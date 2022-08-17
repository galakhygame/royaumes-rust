use crate::auth::error::AccountError;
use crate::auth::event::{AccountEvent, Created, Quantity};
use crate::auth::{account_exist, add_event, load_account, Account, Id};
use crate::{EventDb, MariadDb};
use jsonwebtokens as jwt;
use jsonwebtokens::{encode, AlgorithmID};
use jwt::Algorithm;
use rocket::serde::json::Json;
use rocket::State;
use serde_json::json;
use uuid::Uuid;

use api_account::{AccountCommand, CreateAccount};

#[post("/", format = "json", data = "<command>")]
pub async fn handle_anonymous(
    event_db: &State<EventDb>,
    maria_db: &State<MariadDb>,
    command: Json<AccountCommand>,
) -> Result<String, AccountError> {
    match command.0 {
        AccountCommand::CreateAccount(cmd) => create(event_db, maria_db, cmd).await,
        AccountCommand::AddQuantity(_) => Err(AccountError::Other(
            "cannot add quantity without id".to_string(),
        )),
        AccountCommand::RemoveQuantity(_) => Err(AccountError::Other(
            "cannot remove quantity without id".to_string(),
        )),
    }
}

#[post("/<id>", format = "json", data = "<command>")]
pub async fn handle(
    event_db: &State<EventDb>,
    id: Id,
    command: Json<AccountCommand>,
) -> Result<String, AccountError> {
    match command.0 {
        AccountCommand::CreateAccount(_) => {
            Err(AccountError::Other("cannot create with id".to_string()))
        }
        AccountCommand::AddQuantity(cmd) => add(event_db, id, cmd).await,
        AccountCommand::RemoveQuantity(cmd) => remove(event_db, id, cmd).await,
    }
}

async fn create(
    event_db: &State<EventDb>,
    maria_db: &State<MariadDb>,
    cmd: CreateAccount,
) -> Result<String, AccountError> {
    let mariadb = maria_db.db.clone();
    let uuid = Uuid::new_v4();
    let id = Id::from(uuid);

    let exists = sqlx::query!(
        r#"
SELECT email, pseudo FROM `user` WHERE email like ? OR pseudo LIKE ?;
        "#,
        cmd.email,
        cmd.pseudo,
    )
    .fetch_all(&mariadb)
    .await;

    match exists {
        Err(e) => {
            return Err(AccountError::Other(format!("sql error : {e}")));
        }
        Ok(exists) => {
            if !exists.is_empty() {
                for exist in exists {
                    if exist.email == cmd.email {
                        return Err(AccountError::AlreadyExist(format!(
                            "email {} already exists",
                            exist.email
                        )));
                    }
                    if exist.pseudo == cmd.pseudo {
                        return Err(AccountError::AlreadyExist(format!(
                            "pseudo {} already exists",
                            exist.pseudo
                        )));
                    }
                }
            }
        }
    }

    let new_user = sqlx::query!(
        r#"
INSERT INTO `user`
(`uuid`, `email`, `pseudo`, `password`, `admin`)
VALUES (?, ?, ?, ?, ?);
        "#,
        id.to_string(),
        cmd.email,
        cmd.pseudo,
        cmd.password,
        0
    )
    .execute(&mariadb)
    .await;

    if let Err(e) = new_user {
        return Err(AccountError::Other(format!("sql error : {e}")));
    }

    let db = event_db.db.clone();

    let exist = account_exist(&db, &id).await?;
    if exist {
        return Err(AccountError::AlreadyExist(format!(
            "account {} already exist ( TODO : send to sentry )",
            id
        )));
    }

    let mut events = Vec::new();

    let command = Account::Command(AccountCommand::CreateAccount(CreateAccount {
        pseudo: cmd.pseudo,
        email: "***".to_string(),
        password: "***".to_string(),
    }))
    .to_event_data(None);

    events.push(command.0);

    let created =
        Account::Event(AccountEvent::Created(Created { uuid })).to_event_data(Some(command.1));

    events.push(created.0);

    add_event(&db, &id, events).await?;

    // FIXME dont use unsecure xD
    let alg = Algorithm::new_hmac(AlgorithmID::HS256, "secret").unwrap();
    let header = json!({ "alg": alg.name() });
    let claims = json!({ "uuid": id.to_string() });
    let token = encode(&header, &claims, &alg).unwrap();

    Ok(token)
}

async fn add(event_db: &State<EventDb>, id: Id, nb: usize) -> Result<String, AccountError> {
    let db = event_db.db.clone();

    let account = load_account(&db, &id).await?;

    if account.nb.checked_add(nb).is_none() {
        return Err(AccountError::WrongQuantity(format!(
            "cannot add {} to {}",
            nb, account.nb
        )));
    }

    let payload = Account::Event(AccountEvent::Added(Quantity { nb }));

    add_event(&db, &id, vec![payload.to_event_data(None).0]).await?;

    Ok(format!("added {} in {}", nb, id))
}

async fn remove(event_db: &State<EventDb>, id: Id, nb: usize) -> Result<String, AccountError> {
    let db = event_db.db.clone();

    let account = load_account(&db, &id).await?;

    if nb > account.nb {
        return Err(AccountError::WrongQuantity(format!(
            "cannot remove {} from {}",
            nb, account.nb
        )));
    }

    let payload = Account::Event(AccountEvent::Removed(Quantity { nb }));

    add_event(&db, &id, vec![payload.to_event_data(None).0]).await?;

    Ok(format!("added {} in {}", nb, id))
}
