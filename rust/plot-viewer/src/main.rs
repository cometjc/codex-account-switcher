use anyhow::Result;
use codex_auth::app::App;
use codex_auth::paths::AppPaths;
use codex_auth::store::{AccountStore, StorePlatform};
use codex_auth::usage::UsageService;

fn main() -> Result<()> {
    let paths = AppPaths::detect();
    let store = AccountStore::new(paths.clone(), StorePlatform::detect());
    let usage = UsageService::new(paths.limit_cache_path().to_path_buf(), 300);
    let mut app = App::load(store, usage)?;
    app.run()
}
