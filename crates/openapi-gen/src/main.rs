use std::io::Write;

use api::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() -> anyhow::Result<()> {
    let json = ApiDoc::openapi().to_pretty_json()?;
    std::io::stdout().write_all(format!("{json}\n").as_bytes())?;
    Ok(())
}
