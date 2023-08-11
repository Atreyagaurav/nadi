use anyhow::Result;

pub trait CliAction {
    fn run(self) -> Result<()>;
}
