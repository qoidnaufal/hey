use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Error, Surreal,
};

use crate::auth_model::{LoginRequest, UserData};

#[derive(Clone)]
pub struct Database {
    pub client: Surreal<Client>,
    pub name_space: String,
    pub db_name: String,
}

impl Database {
    pub async fn init(name_space: String, db_name: String) -> Result<Self, Error> {
        let client = Surreal::new::<Ws>("0.0.0.0:8000").await?;
        client
            .signin(Root {
                username: "root",
                password: "root",
            })
            .await?;

        //            -- "hey" --     -- "user-data" --
        client.use_ns(&name_space).use_db(&db_name).await?;

        Ok(Self {
            client,
            name_space,
            db_name,
        })
    }

    pub async fn register_user(
        &self,
        table_name: &'static str,
        uuid: String,
        new_user: UserData,
    ) -> Result<(), Error> {
        let registered_user = self
            .client
            .create::<Option<UserData>>((table_name, uuid.clone()))
            .content(new_user.clone())
            .await;

        match registered_user {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn get_user_by_id(&self, table_name: &'static str, uuid: String) -> Option<UserData> {
        let get_user = self.client.select((table_name, uuid)).await;

        match get_user {
            Ok(maybe_user) => maybe_user,
            Err(_) => None,
        }
    }

    pub async fn get_user_by_query(
        &self,
        table_name: &'static str,
        email: String,
        password: String,
    ) -> Result<Option<String>, Error> {
        match self
            .client
            .query("SELECT uuid FROM type::table($table) WHERE email = $email AND password = $password")
            .bind(("table", table_name))
            .bind(LoginRequest { email, password })
            .await
        {
            Ok(mut maybe_user) => maybe_user.take::<Option<String>>("uuid"),
            Err(err) => Err(err),
        }
    }

    pub async fn _update_user(
        &self,
        table_name: &'static str,
        uuid: String,
        update_data: UserData,
    ) -> Result<Option<UserData>, Error> {
        let update_user = self
            .client
            .update((table_name, uuid.clone()))
            .merge(update_data)
            .await;

        match update_user {
            Ok(maybe_user) => match maybe_user {
                Some(user) => Ok(user),
                None => Err(Error::Db(surrealdb::error::Db::UserRootNotFound {
                    value: uuid,
                })),
            },
            Err(err) => Err(err),
        }
    }

    pub async fn _delete_user(&self, table_name: &'static str, uuid: String) -> Result<(), Error> {
        match self
            .client
            .delete::<Option<UserData>>((table_name, uuid))
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }
}
