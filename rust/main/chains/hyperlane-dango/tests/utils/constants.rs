use std::sync::LazyLock;

use super::user::UserInfo;

pub const EXISTING_CONTRACT: &str = "0x2f3d763027f30db0250de65d037058c8bcbd3352";
pub const NOT_EXISTING_CONTRACT: &str = "0x929a99d0881f07e03d5f91b5ad2a1fc188f64ea1";

pub const EXISTING_USER: &str = "0xcf8c496fb3ff6abd98f2c2b735a0a148fed04b54";
pub const NOT_EXISTING_USER: &str = "0x384ba320f302804a0a03bfc8bb171f35d8b84f01";

pub const EXISTING_COIN: &str = "uusdc";
pub const NOT_EXISTING_COIN: &str = "abcde";

pub const COIN_TYPE: usize = 60;

pub const OWNER: LazyLock<UserInfo> = LazyLock::new(|| {
    UserInfo::new(
        "user_1",
        "junior fault athlete legal inject duty board school anger mesh humor file desk element ticket shop engine paper question love castle ghost bring discover",
        "0xe430fa3a3f13c237fd2f20f8242857cef182b0bd",
    )
});

pub const USER_1: LazyLock<UserInfo> = LazyLock::new(|| {
    UserInfo::new(
        "user_2",
        "visit spend fatigue fork acid junk prize monitor bonus gym frog educate blouse mountain beyond loop nominee argue car shield mixed chunk current force",
        "0x1598c2b6ae4660c4001cd2bc0c96064d24198a82",
    )
});

pub const USER_2: LazyLock<UserInfo> = LazyLock::new(|| {
    UserInfo::new(
        "user_1",
        "impulse youth electric wink tomorrow fruit squirrel practice effort mimic leave year visual calm surge system census tower involve wild symbol coral purchase uniform",
        "0x7f1194e28a176c15ec2fe499fec873ce4756f14",
    )
});
