use diesel::{Queryable, Insertable};

use crate::schema::accounts;

#[derive(Default, Queryable, Insertable)]
pub struct Account {
    pub id: Option<i32>,
    pub client_id: String,
    pub access_token: String,
    pub account_uuid: String,
    pub username: String,
}
