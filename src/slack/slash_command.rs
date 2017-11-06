#[derive(FromForm)]
pub struct Request {
    pub token: String,
    pub team_id: String,
    pub team_domain: String,
    pub channel_id: String,
    pub channel_name: String,
    pub user_id: String,
    pub user_name: String,
    pub text: String,
    pub response_url: String,
    pub trigger_id: String,
}

#[derive(Serialize)]
pub struct Response {
    pub response_type: String,
    pub text: String,
    pub attachments: ResponseAttachments,
}

#[derive(Serialize)]
pub struct ResponseAttachments {
    pub fallback: String,
    pub color: String,
    pub pretext: String,
    pub title: String,
    pub text: String,
    pub fields: Vec<AttachmentField>,
}

#[derive(Serialize)]
pub struct AttachmentField {
    pub title: String,
    pub value: String,
    pub short: bool,
}
