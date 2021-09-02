use anyhow::{Context, Result as AnyResult};

use git2::{Repository, StashFlags, StatusOptions};

pub(crate) trait GitBackend: Sized {
    fn run_in<F, O>(&mut self, id: &str, f: F) -> AnyResult<O>
    where
        F: FnOnce() -> O,
    {
        self.switch_to(id)
            .with_context(|| format!("Failed to checkout to {}", id))?;

        let rslt = f();

        self.switch_back().with_context(|| {
            format!("Failed to switch back to {}", self.source_branch().unwrap())
        })?;

        Ok(rslt)
    }

    fn switch_to(&mut self, id: &str) -> AnyResult<()> {
        if self.has_uncommited_changes() {
            self.stash_push().context("Failed to stash changes")?;
        }

        let branch_name = self
            .head_name()
            .context("Failed to get HEAD name")?
            .expect("Not currently on a branch");

        self.checkout(id)
            .with_context(|| format!("Failed to checkout to {}", id))?;
        self.set_source_branch(branch_name);

        Ok(())
    }

    fn switch_back(&mut self) -> AnyResult<()> {
        let source_branch = self
            .source_branch()
            .expect("Previous branch is not set")
            .to_owned();

        self.checkout(source_branch.as_str())
            .with_context(|| format!("Failed to checkout to {}", source_branch))?;

        if self.has_uncommited_changes() {
            self.stash_pop()
                .context("Failed to restore initial repository state")?;
        }

        Ok(())
    }

    fn head_name(&self) -> AnyResult<Option<String>>;
    fn source_branch(&self) -> Option<&str>;
    fn set_source_branch(&mut self, name: String);

    fn has_uncommited_changes(&self) -> bool;
    fn stash_push(&mut self) -> AnyResult<()>;
    fn stash_pop(&mut self) -> AnyResult<()>;
    fn checkout(&mut self, id: &str) -> AnyResult<()>;
}

/// `CrateRepo` is the structure that contains
/// git related information.
pub(crate) struct CrateRepo {
    // A git repository
    repo: Repository,
    // the source_branch is the branch the current user is working on,
    // in contrast to the target branch (usually main/master), the current user will diff against.
    source_branch: Option<String>,
    has_uncommited_changes: bool,
}

impl GitBackend for CrateRepo {
    fn head_name(&self) -> AnyResult<Option<String>> {
        self.repo
            .head()
            .map(|r| r.name().map(String::from))
            .map_err(Into::into)
    }

    fn source_branch(&self) -> Option<&str> {
        self.source_branch.as_ref().map(AsRef::as_ref)
    }

    fn set_source_branch(&mut self, name: String) {
        assert!(
            self.source_branch.is_none(),
            "set_source_branch is called when a previous branch has already been set"
        );
        self.source_branch = Some(name);
    }

    fn has_uncommited_changes(&self) -> bool {
        self.has_uncommited_changes
    }

    fn stash_push(&mut self) -> AnyResult<()> {
        let stash_options = StashFlags::INCLUDE_UNTRACKED;
        let signature = self
            .repo
            .signature()
            .context("Failed to create user signature")?;

        self.repo
            .stash_save2(&signature, None, Some(stash_options))
            .map(drop)
            .map_err(Into::into)
    }

    fn stash_pop(&mut self) -> AnyResult<()> {
        self.repo
            .stash_pop(0, None)
            .context("Failed to pop the stashed state")
    }

    fn checkout(&mut self, id: &str) -> AnyResult<()> {
        let (obj, reference) = self
            .repo
            .revparse_ext(id)
            .with_context(|| format!("Failed to get object corresponding to {}", id))?;

        self.repo
            .checkout_tree(&obj, None)
            .with_context(|| format!("Failed to change the tree to {}", id))?;

        match reference {
            Some(gref) => self
                .repo
                .set_head(gref.name().expect("Branch name must be UTF-8"))
                .with_context(|| format!("Failed to set head to {}", gref.name().unwrap()))?,

            None => self.repo.set_head_detached(obj.id())?,
        }

        Ok(())
    }
}

impl CrateRepo {
    pub fn new() -> AnyResult<Self> {
        let repo = Repository::open_from_env().context("Failed to open repository")?;

        let has_uncommited_changes = !repo
            .statuses(Some(StatusOptions::new().include_untracked(true)))
            .context("Failed to get current repository status")?
            .is_empty();

        Ok(Self {
            repo,
            source_branch: None,
            has_uncommited_changes,
        })
    }
}

#[cfg(test)]
use std::{cell::RefCell, rc::Rc};

#[cfg(test)]
#[allow(clippy::type_complexity)]
struct FakeRepo<'a> {
    on_head_name: Box<dyn Fn(&FakeRepo) -> AnyResult<Option<String>> + 'a>,
    on_source_branch: Box<dyn for<'b> Fn(&'b FakeRepo) -> Option<&'b str> + 'a>,
    on_set_source_branch: Rc<dyn Fn(&mut FakeRepo<'a>, &str) + 'a>,
    on_has_uncommited_changes: Box<dyn Fn(&FakeRepo<'a>) -> bool + 'a>,
    on_stash_push: Rc<dyn Fn(&mut FakeRepo<'a>) -> AnyResult<()> + 'a>,
    on_stash_pop: Rc<dyn Fn(&mut FakeRepo<'a>) -> AnyResult<()> + 'a>,
    on_checkout: Rc<dyn Fn(&mut FakeRepo<'a>, &str) -> AnyResult<()> + 'a>,

    actions: RefCell<Vec<FakeRepoCall>>,
}

#[cfg(test)]
impl<'a> GitBackend for FakeRepo<'a> {
    fn current() -> AnyResult<Self> {
        Ok(FakeRepo::new())
    }

    fn head_name(&self) -> AnyResult<Option<String>> {
        self.add_action(FakeRepoCall::HeadName);
        (self.on_head_name)(self)
    }

    fn source_branch(&self) -> Option<&str> {
        self.add_action(FakeRepoCall::PreviousBranch);
        (self.on_source_branch)(self)
    }

    fn set_source_branch(&mut self, name: String) {
        self.add_action(FakeRepoCall::SetPreviousBranch(name));
        self.on_set_source_branch.clone()(self, name)
    }

    fn needs_stash(&self) -> bool {
        self.add_action(FakeRepoCall::HasUncommitedChanges);
        (self.on_has_uncommited_changes)(self)
    }

    fn stash_push(&mut self) -> AnyResult<()> {
        self.add_action(FakeRepoCall::StashPush);
        self.on_stash_push.clone()(self)
    }

    fn stash_pop(&mut self) -> AnyResult<()> {
        self.add_action(FakeRepoCall::StashPop);
        self.on_stash_pop.clone()(self)
    }

    fn checkout(&mut self, id: &str) -> AnyResult<()> {
        self.add_action(FakeRepoCall::CheckoutTo(id.to_owned()));
        self.on_checkout.clone()(self, id)
    }
}

#[cfg(test)]
impl<'a> FakeRepo<'a> {
    fn new() -> FakeRepo<'a> {
        FakeRepo {
            on_head_name: Box::new(|_| panic!("`on_head_name` is called but not set")),
            on_source_branch: Box::new(|_| panic!("`on_source_branch` is called but not set")),
            on_set_source_branch: Rc::new(|_, _| {
                panic!("`on_set_source_branch` is called but not set")
            }),
            on_has_uncommited_changes: Box::new(|_| {
                panic!("`on_has_uncommited_changes` is called but not set")
            }),
            on_stash_push: Rc::new(|_| panic!("`on_stash_push` is called but not set")),
            on_stash_pop: Rc::new(|_| panic!("`on_stash_pop` is called but not set")),
            on_checkout: Rc::new(|_, _| panic!("`on_checkout` is called but not set")),

            actions: RefCell::new(Vec::new()),
        }
    }

    fn on_head_name(
        mut self,
        f: impl Fn(&FakeRepo) -> AnyResult<Option<String>> + 'a,
    ) -> FakeRepo<'a> {
        self.on_head_name = Box::new(f);
        self
    }

    fn on_source_branch(
        mut self,
        f: impl for<'b> Fn(&'b FakeRepo) -> Option<&'b str> + 'a,
    ) -> FakeRepo<'a> {
        self.on_source_branch = Box::new(f);
        self
    }

    fn on_set_source_branch(mut self, f: impl Fn(&mut FakeRepo<'a>, String) + 'a) -> FakeRepo<'a> {
        self.on_set_source_branch = Rc::new(f);
        self
    }

    fn on_needs_stash(mut self, f: impl Fn(&FakeRepo<'a>) -> bool + 'a) -> FakeRepo<'a> {
        self.on_needs_stash = Box::new(f);
        self
    }

    fn on_stash_push(
        mut self,
        f: impl Fn(&mut FakeRepo<'a>) -> AnyResult<()> + 'a,
    ) -> FakeRepo<'a> {
        self.on_stash_push = Rc::new(f);
        self
    }

    fn on_stash_pop(mut self, f: impl Fn(&mut FakeRepo<'a>) -> AnyResult<()> + 'a) -> FakeRepo<'a> {
        self.on_stash_pop = Rc::new(f);
        self
    }

    fn on_checkout(
        mut self,
        f: impl Fn(&mut FakeRepo<'a>, &str) -> AnyResult<()> + 'a,
    ) -> FakeRepo<'a> {
        self.on_checkout = Rc::new(f);
        self
    }

    fn add_action(&self, a: FakeRepoCall) {
        self.actions.borrow_mut().push(a);
    }
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
enum FakeRepoCall {
    HeadName,
    PreviousBranch,
    SetPreviousBranch(String),
    HasUncommitedChanges,
    StashPush,
    StashPop,
    CheckoutTo(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    mod switch_to_default_impl {
        use super::*;

        #[test]
        fn when_stash() {
            let mut repo = FakeRepo::new()
                .on_needs_stash(|_| true)
                .on_stash_push(|_| Ok(()))
                .on_head_name(|_| Ok(Some("foo".to_owned())))
                .on_checkout(|_, _| Ok(()))
                .on_set_source_branch(|_, _| ());

            repo.switch_to("bar").unwrap();

            let expected_calls = [
                FakeRepoCall::HasUncommitedChanges,
                FakeRepoCall::StashPush,
                FakeRepoCall::HeadName,
                FakeRepoCall::CheckoutTo("bar".to_owned()),
                FakeRepoCall::SetPreviousBranch("foo".to_owned()),
            ];

            assert_eq!(repo.actions.borrow().as_ref(), expected_calls);
        }

        #[test]
        fn when_non_stash() {
            let mut repo = FakeRepo::new()
                .on_needs_stash(|_| false)
                .on_head_name(|_| Ok(Some("bar".to_owned())))
                .on_checkout(|_, _| Ok(()))
                .on_set_source_branch(|_, _| ());

            repo.switch_to("baz").unwrap();

            let expected_calls = [
                FakeRepoCall::NeedsStash,
                FakeRepoCall::HeadName,
                FakeRepoCall::CheckoutTo("baz".to_owned()),
                FakeRepoCall::SetPreviousBranch("bar".to_owned()),
            ];

            assert_eq!(repo.actions.borrow().as_ref(), expected_calls);
        }
    }

    mod go_back_default_impl {
        use super::*;

        #[test]
        fn when_stash() {
            let mut repo = FakeRepo::new()
                .on_source_branch(|_| Some("bar"))
                .on_checkout(|_, _| Ok(()))
                .on_needs_stash(|_| true)
                .on_stash_pop(|_| Ok(()));

            repo.switch_back().unwrap();

            let expected_calls = [
                FakeRepoCall::PreviousBranch,
                FakeRepoCall::CheckoutTo("bar".to_owned()),
                FakeRepoCall::NeedsStash,
                FakeRepoCall::StashPop,
            ];

            assert_eq!(repo.actions.borrow().as_ref(), expected_calls);
        }

        #[test]
        fn when_non_stash() {
            let mut repo = FakeRepo::new()
                .on_source_branch(|_| Some("bar"))
                .on_checkout(|_, _| Ok(()))
                .on_needs_stash(|_| false);

            repo.switch_back().unwrap();

            let expected_calls = [
                FakeRepoCall::PreviousBranch,
                FakeRepoCall::CheckoutTo("bar".to_owned()),
                FakeRepoCall::NeedsStash,
            ];

            assert_eq!(repo.actions.borrow().as_ref(), expected_calls);
        }
    }
}
