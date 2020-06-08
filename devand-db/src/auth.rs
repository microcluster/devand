use crate::{models, schema, Error};
use argon2::{self, Config};
use diesel::pg::PgConnection;
use diesel::prelude::*;

fn generate_salt() -> [u8; 16] {
    use rand::Rng;
    rand::thread_rng().gen()
}

fn encode_password(password: &str) -> String {
    let password = password.as_bytes();
    let salt = generate_salt();
    let config = Config::default();
    argon2::hash_encoded(password, &salt, &config).unwrap()
}

fn verify_password(hash: &str, password: &str) -> bool {
    let password = password.as_bytes();
    argon2::verify_encoded(&hash, password).unwrap()
}

pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub struct JoinData {
    pub username: String,
    pub email: String,
    pub password: String,
}

pub fn join(join_data: JoinData, conn: &PgConnection) -> Result<(), Error> {
    use schema::auth;
    use schema::users;

    let JoinData {
        username,
        email,
        password,
    } = join_data;

    let settings = devand_core::UserSettings::default();
    let settings = serde_json::to_value(settings).map_err(|_| Error::Unknown)?;

    let new_user = models::NewUser {
        username,
        email,
        settings,
    };

    let user: models::User = diesel::insert_into(users::table)
        .values(&new_user)
        .get_result(conn)
        .map_err(|err| {
            dbg!(err);
            Error::Unknown
        })?;

    let enc_password = encode_password(&password);

    let new_auth = models::NewAuth {
        user_id: user.id,
        enc_password,
    };

    let ok = diesel::insert_into(auth::table)
        .values(&new_auth)
        .execute(conn)
        .map_err(|err| {
            dbg!(err);
            Error::Unknown
        })?;

    assert_eq!(ok, 1);

    Ok(())
}

pub fn login(credentials: Credentials, conn: &PgConnection) -> Result<(i32, String), Error> {
    let Credentials { username, password } = credentials;

    let auth: models::Auth = schema::login::table
        .filter(schema::login::dsl::username.like(username))
        .first(conn)
        .map_err(|err| {
            dbg!(err);
            Error::Unknown
        })?;

    if verify_password(&auth.enc_password, &password) {
        Ok((auth.user_id, auth.username))
    } else {
        Err(Error::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_ok() {
        let password = "password";
        let hash = encode_password(password);

        assert!(verify_password(&hash, password));
    }
}