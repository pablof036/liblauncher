pub mod models;

use diesel::insert_into;
use diesel::prelude::*;
use crate::error::{Result};
use crate::store::models::Account;
use crate::schema::accounts::dsl::*;

fn establish_connection() -> Result<SqliteConnection> {
    Ok(SqliteConnection::establish("launcher.db")?)
}

pub fn get_accounts() -> Result<Vec<Account>> {
    Ok(accounts.load::<Account>(&establish_connection()?)?)
}

pub fn store_account(account: &Account) -> Result<()> {
    insert_into(accounts)
        .values(account)
        .execute(&establish_connection()?)?;
    Ok(())
}