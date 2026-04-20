mod protocol;
mod server;
mod tools;

use crate::bootstrap;
pub use server::handle_json_rpc_value;

pub fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Runtime::new()?;
    let state = runtime.block_on(bootstrap::initialize_app_state())?;
    let mut server = server::McpServer::new(state);
    server.serve(&runtime)?;
    Ok(())
}
