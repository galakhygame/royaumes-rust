#![feature(future_join)]

use crate::multiple::build::{BuildCommand, BuildState, BuildingCreate};
use crate::multiple::flow::{AskPayment, Payment};
use crate::multiple::gold::GoldState;
use crate::multiple::worker::WorkerState;
use crate::multiple::Cost;
use eventstore::Client as EventClient;
use state_repository::{ModelKey, StateRepository};
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use workflow::Distant;

mod multiple;

#[tokio::test]
async fn multiple_state_case() {
    let repo = get_repository();

    let key = ModelKey::new("tower_test".to_string(), Uuid::new_v4().to_string());

    let key_bank = ModelKey::new("bank_test".to_string(), Uuid::new_v4().to_string());

    let key_citizen = ModelKey::new("citizen_test".to_string(), Uuid::new_v4().to_string());

    <GoldState as Distant<Payment>>::listen(repo.clone());
    <BuildState as Distant<AskPayment>>::listen(repo.clone());

    sleep(Duration::from_secs(1)).await;

    let create = BuildingCreate {
        cost: Cost {
            gold: 322,
            worker: 42,
        },
        bank: key_bank.clone(),
        citizen: key_citizen.clone(),
    };

    let cost = Cost {
        gold: 322,
        worker: 42,
    };

    let build = repo
        .add_command::<BuildState>(&key, BuildCommand::Create(create), None)
        .await
        .unwrap();

    assert_eq!(
        build,
        (BuildState {
            cost,
            allocated: Default::default(),
            built: false,
            citizen: Some(key_citizen.clone()),
            bank: Some(key_bank.clone()),
            position: None,
        })
    );

    sleep(Duration::from_secs(1)).await;

    let state = repo.get_model::<BuildState>(&key).await.unwrap();

    let all_allocated = Cost {
        gold: 322,
        worker: 42,
    };

    assert_eq!(
        state,
        BuildState {
            cost,
            allocated: all_allocated,
            built: false,
            citizen: Some(key_citizen.clone()),
            bank: Some(key_bank.clone()),
            position: Some(7),
        }
    );

    let worker_state = repo.get_model::<WorkerState>(&key_citizen).await.unwrap();

    assert_eq!(
        worker_state,
        WorkerState {
            nb: 58,
            position: Some(2),
        }
    );

    let gold_state = repo.get_model::<GoldState>(&key_bank).await.unwrap();

    assert_eq!(
        gold_state,
        GoldState {
            nb: 678,
            position: Some(2),
        }
    );

    sleep(Duration::from_secs(3)).await;

    let state = repo.get_model::<BuildState>(&key).await.unwrap();
    let worker_freed = Cost {
        gold: 322,
        worker: 0,
    };

    assert_eq!(
        state,
        BuildState {
            cost,
            allocated: worker_freed,
            built: true,
            position: Some(12),
            citizen: Some(key_citizen.clone()),
            bank: Some(key_bank.clone()),
        }
    );

    let worker_state = repo.get_model::<WorkerState>(&key_citizen).await.unwrap();

    assert_eq!(
        worker_state,
        WorkerState {
            nb: 100,
            position: Some(5),
        }
    );
    let gold_state = repo.get_model::<GoldState>(&key_bank).await.unwrap();

    assert_eq!(
        gold_state,
        GoldState {
            nb: 678,
            position: Some(2),
        }
    );
}

fn get_repository() -> StateRepository {
    let settings = "esdb://admin:changeit@localhost:2113?tls=false&tlsVerifyCert=false"
        .to_string()
        .parse()
        .unwrap();
    let event_db = EventClient::new(settings).unwrap();

    let cache_db = redis::Client::open("redis://localhost:6379/").unwrap();

    let repo = StateRepository::new(event_db, cache_db);

    repo
}
