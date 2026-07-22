use deli::Model;
use keystone::identity::{DhPublicKey, SigningPublicKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct WebFriend {
    #[deli(key)]
    sign_pub: SigningPublicKey,
    dh_pub: DhPublicKey,
    nickname: String,
    pairwise_root: [u8; 32],
}

impl From<keystone::Friend> for WebFriend {
    fn from(friend: keystone::Friend) -> Self {
        WebFriend {
            sign_pub: friend.public.sign_pub,
            dh_pub: friend.public.dh_pub,
            nickname: friend.nickname,
            pairwise_root: friend.pairwise_root,
        }
    }
}

impl From<WebFriend> for keystone::Friend {
    fn from(web_friend: WebFriend) -> Self {
        keystone::Friend {
            public: keystone::PublicIdentity {
                sign_pub: web_friend.sign_pub,
                dh_pub: web_friend.dh_pub,
            },
            nickname: web_friend.nickname,
            pairwise_root: web_friend.pairwise_root,
        }
    }
}
