#[macro_use]
extern crate diesel;

use lazy_static::lazy_static;
pub mod auth;
mod game_profile;
mod resources;
pub mod error;

mod store;
mod schema;



lazy_static! {
    static ref client: reqwest::Client = reqwest::Client::new();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
