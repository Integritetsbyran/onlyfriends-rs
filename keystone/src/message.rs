use serde::{Deserialize, Serialize};

use crate::Profile;
use crate::post::PostContent;
use crate::response::{ResponseInner, ResponseRebroadcast};


#[derive(Serialize, Deserialize)]
pub enum Message {
    Post(PostContent),
    Profile(Profile),
    Response(ResponseInner),
    Rebroadcast(ResponseRebroadcast),
}
