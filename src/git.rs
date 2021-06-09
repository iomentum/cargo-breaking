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
            format!(
                "Failed to switch back to {}",
                self.previous_branch().unwrap()
            )
        })?;

        Ok(rslt)
    }

    fn switch_to(&mut self, id: &str) -> AnyResult<()> {
        if self.needs_stash() {
            self.stash_push().context("Failed to stash changes")?;
        }

        let branch_name = self
            .head_name()
            .context("Failed to get HEAD name")?
            .expect("Not currently on a branch");

        self.checkout_to(id)
            .with_context(|| format!("Failed to checkout to {}", id))?;
        self.set_previous_branch(branch_name.as_str());

        Ok(())
    }

    fn switch_back(&mut self) -> AnyResult<()> {
        let previous_branch = self
            .previous_branch()
            .expect("Previous branch is not set")
            .to_owned();

        self.checkout_to(previous_branch.as_str())
            .with_context(|| format!("Failed to checkout to {}", previous_branch))?;

        if self.needs_stash() {
            self.stash_pop()
                .context("Failed to restore initial repository state")?;
        }

        Ok(())
    }

    fn current() -> AnyResult<Self>;

    fn head_name(&self) -> AnyResult<Option<String>>;
    fn previous_branch(&self) -> Option<&str>;
    fn set_previous_branch(&mut self, name: &str);

    fn needs_stash(&self) -> bool;
    fn stash_push(&mut self) -> AnyResult<()>;
    fn stash_pop(&mut self) -> AnyResult<()>;
    fn checkout_to(&mut self, id: &str) -> AnyResult<()>;
}

pub(crate) struct CrateRepo {
    repo: Repository,
    previous_branch_name: Option<String>,
    needs_stash: bool,
}

impl GitBackend for CrateRepo {
    fn current() -> AnyResult<CrateRepo> {
        let repo = Repository::open_from_env().context("Failed to open repository")?;
        let needs_stash =
            CrateRepo::needs_stash(&repo).context("Failed to determine if stash is needed")?;

        Ok(CrateRepo {
            repo,
            previous_branch_name: None,
            needs_stash,
        })
    }

    fn head_name(&self) -> AnyResult<Option<String>> {
        self.repo
            .head()
            .map(|r| r.name().map(String::from))
            .map_err(Into::into)
    }

    fn previous_branch(&self) -> Option<&str> {
        self.previous_branch_name.as_ref().map(AsRef::as_ref)
    }

    fn set_previous_branch(&mut self, name: &str) {
        assert!(
            self.previous_branch_name.is_none(),
            "set_previous_branch is called when a previous branch is already set"
        );
        self.previous_branch_name = Some(name.to_owned());
    }

    fn needs_stash(&self) -> bool {
        self.needs_stash
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

    fn checkout_to(&mut self, id: &str) -> AnyResult<()> {
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
    fn needs_stash(repo: &Repository) -> AnyResult<bool> {
        let mut options = StatusOptions::new();
        let options = options.include_untracked(true);

        let statuses = repo
            .statuses(Some(options))
            .context("Failed to get current repository status")?;

        Ok(!statuses.is_empty())
    }
}

#[cfg(test)]
use std::{cell::RefCell, rc::Rc};

#[cfg(test)]
#[allow(clippy::type_complexity)]
struct FakeRepo<'a> {
    on_head_name: Box<dyn Fn(&FakeRepo) -> AnyResult<Option<String>> + 'a>,
    on_previous_branch: Box<dyn for<'b> Fn(&'b FakeRepo) -> Option<&'b str> + 'a>,
    on_set_previous_branch: Rc<dyn Fn(&mut FakeRepo<'a>, &str) + 'a>,
    on_needs_stash: Box<dyn Fn(&FakeRepo<'a>) -> bool + 'a>,
    on_stash_push: Rc<dyn Fn(&mut FakeRepo<'a>) -> AnyResult<()> + 'a>,
    on_stash_pop: Rc<dyn Fn(&mut FakeRepo<'a>) -> AnyResult<()> + 'a>,
    on_checkout_to: Rc<dyn Fn(&mut FakeRepo<'a>, &str) -> AnyResult<()> + 'a>,

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

    fn previous_branch(&self) -> Option<&str> {
        self.add_action(FakeRepoCall::PreviousBranch);
        (self.on_previous_branch)(self)
    }

    fn set_previous_branch(&mut self, name: &str) {
        self.add_action(FakeRepoCall::SetPreviousBranch(name.to_owned()));
        self.on_set_previous_branch.clone()(self, name)
    }

    fn needs_stash(&self) -> bool {
        self.add_action(FakeRepoCall::NeedsStash);
        (self.on_needs_stash)(self)
    }

    fn stash_push(&mut self) -> AnyResult<()> {
        self.add_action(FakeRepoCall::StashPush);
        self.on_stash_push.clone()(self)
    }

    fn stash_pop(&mut self) -> AnyResult<()> {
        self.add_action(FakeRepoCall::StashPop);
        self.on_stash_pop.clone()(self)
    }

    fn checkout_to(&mut self, id: &str) -> AnyResult<()> {
        self.add_action(FakeRepoCall::CheckoutTo(id.to_owned()));
        self.on_checkout_to.clone()(self, id)
    }
}

#[cfg(test)]
impl<'a> FakeRepo<'a> {
    fn new() -> FakeRepo<'a> {
        FakeRepo {
            on_head_name: Box::new(|_| panic!("`on_head_name` is called but not set")),
            on_previous_branch: Box::new(|_| panic!("`on_previous_branch` is called but not set")),
            on_set_previous_branch: Rc::new(|_, _| {
                panic!("`on_set_previous_branch` is called but not set")
            }),
            on_needs_stash: Box::new(|_| panic!("`on_needs_stash` is called but not set")),
            on_stash_push: Rc::new(|_| panic!("`on_stash_push` is called but not set")),
            on_stash_pop: Rc::new(|_| panic!("`on_stash_pop` is called but not set")),
            on_checkout_to: Rc::new(|_, _| panic!("`on_checkout_to` is called but not set")),

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

    fn on_previous_branch(
        mut self,
        f: impl for<'b> Fn(&'b FakeRepo) -> Option<&'b str> + 'a,
    ) -> FakeRepo<'a> {
        self.on_previous_branch = Box::new(f);
        self
    }

    fn on_set_previous_branch(mut self, f: impl Fn(&mut FakeRepo<'a>, &str) + 'a) -> FakeRepo<'a> {
        self.on_set_previous_branch = Rc::new(f);
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

    fn on_checkout_to(
        mut self,
        f: impl Fn(&mut FakeRepo<'a>, &str) -> AnyResult<()> + 'a,
    ) -> FakeRepo<'a> {
        self.on_checkout_to = Rc::new(f);
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
    NeedsStash,
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
                .on_checkout_to(|_, _| Ok(()))
                .on_set_previous_branch(|_, _| ());

            repo.switch_to("bar").unwrap();

            let expected_calls = [
                FakeRepoCall::NeedsStash,
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
                .on_checkout_to(|_, _| Ok(()))
                .on_set_previous_branch(|_, _| ());

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
                .on_previous_branch(|_| Some("bar"))
                .on_checkout_to(|_, _| Ok(()))
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
                .on_previous_branch(|_| Some("bar"))
                .on_checkout_to(|_, _| Ok(()))
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
