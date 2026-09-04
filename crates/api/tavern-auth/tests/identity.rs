//! 身份到用户目录的映射。

use tavern_auth::Identity;
use tavern_storage::DataRoot;

#[test]
fn default_identity_maps_to_its_own_dir() {
    let id = Identity::default_user();
    let dirs = id.dirs(&DataRoot::new("/data"));
    assert!(dirs.root().ends_with("default-user"));
}
