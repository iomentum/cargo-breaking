use anyhow::Result as AnyResult;

use git2::{Repository, StashFlags, StatusOptions};

pub(crate) static DEFAULT_BRANCH_NAME: &str = "main";

pub(crate) trait GitBackend: Sized {
    fn switch_to(&mut self, id: &str) -> AnyResult<()> {
        if self.needs_stash() {
            self.stash_push()?;
        }

        let branch_name = self.head_name()?.expect("Not currently on a branch");

        self.checkout_to(id)?;
        self.set_previous_branch(branch_name.as_str());

        Ok(())
    }

    fn switch_back(&mut self) -> AnyResult<()> {
        let previous_branch = self
            .previous_branch()
            .expect("Previous branch is not set")
            .to_owned();

        self.checkout_to(previous_branch.as_str())?;

        if self.needs_stash() {
            self.stash_pop()?;
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
        let repo = Repository::open_from_env()?;
        let needs_stash = CrateRepo::needs_stash(&repo)?;

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
        self.repo
            .stash_save2(&self.repo.signature()?, None, Some(stash_options))
            .map(drop)
            .map_err(Into::into)
    }

    fn stash_pop(&mut self) -> AnyResult<()> {
        self.repo.stash_pop(0, None).map_err(Into::into)
    }

    fn checkout_to(&mut self, id: &str) -> AnyResult<()> {
        let (obj, reference) = self.repo.revparse_ext(id)?;

        self.repo.checkout_tree(&obj, None)?;

        match reference {
            Some(gref) => self
                .repo
                .set_head(gref.name().expect("Branch name must be UTF-8"))?,
            None => self.repo.set_head_detached(obj.id())?,
        }

        Ok(())
    }
}

impl CrateRepo {
    fn needs_stash(repo: &Repository) -> AnyResult<bool> {
        let mut options = StatusOptions::new();
        let options = options.include_untracked(true);

        let statuses = repo.statuses(Some(options))?;

        Ok(!statuses.is_empty())
    }
}
