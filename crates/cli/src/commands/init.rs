use crate::errors::Error;
use miden_client::{crypto::FeltRng, Client};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Initialize the publisher account")]
pub struct InitCmd {}

impl InitCmd {
    pub async fn call(self, client: &mut Client<impl FeltRng>) -> Result<(), Error> {
        todo!();
    }
}
