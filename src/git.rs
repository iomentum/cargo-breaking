use anyhow::{anyhow, Result as AnyResult};

use git2::{Branch, BranchType, Repository};

static DEFAULT_BRANCH_NAME: &str = "main";

pub(crate) struct CrateRepo {
    repo: Repository,
    previous_branch_name: Option<String>,
}

impl CrateRepo {
    pub(crate) fn current() -> AnyResult<CrateRepo> {
        let repo = Repository::open_from_env()?;

        Ok(CrateRepo {
            repo,
            previous_branch_name: None,
        })
    }

    pub(crate) fn checkout_to_main(&mut self) -> AnyResult<()> {
        self.stash_push()?;

        let branch_name = self.current_branch_name()?.name()?.unwrap().to_owned();

        self.checkout_to(DEFAULT_BRANCH_NAME)?;

        self.previous_branch_name = Some(branch_name);

        Ok(())
    }

    pub(crate) fn checkout_to_previous_branch(&mut self) -> AnyResult<()> {
        let previous_branch_name = self
            .previous_branch_name
            .clone()
            .expect("checkout_to_previous_branch must be called after checkout_to_main");

        self.checkout_to(previous_branch_name.as_str())?;

        self.stash_pop()?;

        Ok(())
    }

    fn current_branch_name(&self) -> AnyResult<Branch> {
        self.repo
            .branches(Some(BranchType::Local))?
            .flatten()
            .map(|(b, _)| b)
            .find(|b| b.is_head())
            .ok_or_else(|| anyhow!("No branch is selected"))
    }

    fn stash_push(&mut self) -> AnyResult<()> {
        self.repo
            .stash_save2(&self.repo.signature()?, None, None)
            .map(drop)
            .map_err(Into::into)
    }

    fn stash_pop(&mut self) -> AnyResult<()> {
        self.repo.stash_pop(0, None).map_err(Into::into)
    }

    fn checkout_to(&mut self, name: &str) -> AnyResult<()> {
        let (obj, reference) = self.repo.revparse_ext(name)?;

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
