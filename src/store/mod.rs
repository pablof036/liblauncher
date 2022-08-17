pub mod models;

use diesel::insert_into;
use diesel::prelude::*;
use crate::embedded_migrations;
use crate::error::{Result};
use crate::path_with_launcher;
use crate::store::models::Account;
use crate::schema::accounts::dsl::*;

fn establish_connection() -> Result<SqliteConnection> {
    let r = std::fs::create_dir_all(&crate::config.launcher_path);
        
    let connection = SqliteConnection::establish(&path_with_launcher("launcher.db"))?;     
    
    if let Ok(_) = r {
        init_store(&connection)?;
    }

    Ok(connection)
}

pub fn init_store(connection: &SqliteConnection) -> Result<()> {
    //TODO: delete this unwrap
    embedded_migrations::run(connection).unwrap();
    Ok(())
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