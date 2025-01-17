use crate::helpers::app::test_app_with_login;
use crate::helpers::db::reset;
use crate::helpers::http_client::{PostPayload, TestHttpClient};
use crate::helpers::{read_json, wait_for_stubr};
use actix_web::test;
use anyhow::Error;
use rumba::db::model::{DocumentMetadata, NotificationDataInsert};
use rumba::db::{self, Pool};
use serde_json::json;

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_get_notifications() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;

    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    //Database was reset so we can naively assume user_id = 1.
    let _ids = create_notifications(&pool, 1, 100).await;
    let mut offset = 0;
    let mut limit = 10;

    let mut res = logged_in_client
        .get(
            format!(
                "/api/v1/plus/notifications/?offset={}&limit={}",
                offset, limit
            )
            .as_str(),
            None,
        )
        .await;
    assert_eq!(res.response().status(), 200);

    let mut res_json = read_json(res).await;
    assert_eq!(res_json["items"].as_array().unwrap().len(), 10);
    assert_eq!(
        res_json["items"].as_array().unwrap()[0]["title"],
        "Test title 99"
    );
    assert_eq!(
        res_json["items"].as_array().unwrap()[0]["url"],
        "https://developer.allizom.org/99"
    );
    assert_eq!(
        res_json["items"].as_array().unwrap()[0]["text"],
        "Test text 99"
    );
    assert_eq!(res_json["items"].as_array().unwrap()[0]["starred"], false);
    assert_eq!(res_json["items"].as_array().unwrap()[0]["deleted"], false);
    assert_eq!(
        res_json["items"].as_array().unwrap()[0]["id"],
        _ids.last().unwrap().to_owned()
    );
    assert_eq!(res_json["items"].as_array().unwrap()[0]["read"], false);
    assert!(res_json["items"].as_array().unwrap()[0]["parents"].is_null());

    assert_eq!(
        res_json["items"].as_array().unwrap()[9]["title"],
        "Test title 90"
    );

    offset = 94;
    limit = 7;
    res = logged_in_client
        .get(
            format!(
                "/api/v1/plus/notifications/?offset={}&limit={}",
                offset, limit
            )
            .as_str(),
            None,
        )
        .await;

    res_json = read_json(res).await;
    assert_eq!(res_json["items"].as_array().unwrap().len(), 6);
    assert_eq!(
        res_json["items"].as_array().unwrap()[0]["title"],
        "Test title 5"
    );
    assert_eq!(
        res_json["items"].as_array().unwrap()[4]["title"],
        "Test title 1"
    );

    offset = 200;
    limit = 10;
    res = logged_in_client
        .get(
            format!(
                "/api/v1/plus/notifications/?offset={}&limit={}",
                offset, limit
            )
            .as_str(),
            None,
        )
        .await;
    res_json = read_json(res).await;
    assert_eq!(res_json["items"].as_array().unwrap().len(), 0);
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_mark_all_read() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    //Database was reset so we can naively assume user_id = 1.
    let _ids = create_notifications(&pool, 1, 100).await;

    let mut res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=100", None)
        .await;
    let mut res_json = read_json(res).await;
    let mut items = res_json["items"].as_array().unwrap();
    items.iter().for_each(|val| assert_eq!(val["read"], false));

    res = logged_in_client
        .post("/api/v1/plus/notifications/all/mark-as-read/", None, None)
        .await;
    assert_eq!(res.status(), 200);
    res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=100", None)
        .await;
    res_json = read_json(res).await;
    items = res_json["items"].as_array().unwrap();
    assert_eq!(items.len(), 100);
    items.iter().for_each(|val| assert_eq!(val["read"], true));
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]
async fn test_mark_id_as_read() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    //Database was reset so we can naively assume user_id = 1.
    let _ids = create_notifications(&pool, 1, 100).await;

    let mut res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    let mut res_json = read_json(res).await;
    let mut items = res_json["items"].as_array().unwrap();

    assert_eq!(items[0]["read"], false);
    assert_eq!(items[1]["read"], false);
    assert_eq!(items[2]["read"], false);

    res = logged_in_client
        .post("/api/v1/plus/notifications/99/mark-as-read/", None, None)
        .await;
    assert_eq!(res.status(), 200);
    res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    res_json = read_json(res).await;
    items = res_json["items"].as_array().unwrap();
    assert_eq!(items[0]["read"], false);
    assert_eq!(items[1]["read"], true);
    assert_eq!(items[2]["read"], false);

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]

async fn test_star_unstar_many() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    //Database was reset so we can naively assume user_id = 1.
    let _ids = create_notifications(&pool, 1, 100).await;

    let mut res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    let mut res_json = read_json(res).await;
    let items = res_json["items"].as_array().unwrap();
    items
        .iter()
        .for_each(|val| assert_eq!(val["starred"], false));
    res = logged_in_client
        .post(
            "/api/v1/plus/notifications/star-ids/",
            None,
            Some(PostPayload::Json(json!({"ids": [98,99,100]}))),
        )
        .await;

    assert_eq!(res.status(), 200);
    res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    res_json = read_json(res).await;
    let items = res_json["items"].as_array().unwrap();
    assert_eq!(items[0]["starred"], true);
    assert_eq!(items[1]["starred"], true);
    assert_eq!(items[2]["starred"], true);

    logged_in_client
        .post(
            "/api/v1/plus/notifications/unstar-ids/",
            None,
            Some(PostPayload::Json(json!({"ids": [98,99]}))),
        )
        .await;
    res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    res_json = read_json(res).await;
    let items = res_json["items"].as_array().unwrap();
    assert_eq!(items[0]["starred"], true);
    assert_eq!(items[1]["starred"], false);
    assert_eq!(items[2]["starred"], false);
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]

async fn test_toggle_starred() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    //Database was reset so we can naively assume user_id = 1.
    let _ids = create_notifications(&pool, 1, 100).await;

    let mut res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    let mut res_json = read_json(res).await;
    let items = res_json["items"].as_array().unwrap();
    items
        .iter()
        .for_each(|val| assert_eq!(val["starred"], false));
    res = logged_in_client
        .post("/api/v1/plus/notifications/99/toggle-starred/", None, None)
        .await;

    assert_eq!(res.status(), 200);
    res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    res_json = read_json(res).await;
    let items = res_json["items"].as_array().unwrap();
    assert_eq!(items[0]["starred"], false);
    assert_eq!(items[1]["starred"], true);
    assert_eq!(items[2]["starred"], false);

    logged_in_client
        .post("/api/v1/plus/notifications/99/toggle-starred/", None, None)
        .await;

    res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    res_json = read_json(res).await;
    let items = res_json["items"].as_array().unwrap();
    assert_eq!(items[0]["starred"], false);
    assert_eq!(items[1]["starred"], false);
    assert_eq!(items[2]["starred"], false);
    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]

async fn test_delete_and_undo() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    //Database was reset so we can naively assume user_id = 1.
    let _ids = create_notifications(&pool, 1, 10).await;

    let mut res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    let mut res_json = read_json(res).await;
    assert_eq!(res_json["items"].as_array().unwrap().len(), 10);
    res = logged_in_client
        .post("/api/v1/plus/notifications/9/delete/", None, None)
        .await;
    assert_eq!(res.status(), 200);
    res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    res_json = read_json(res).await;
    let items = res_json["items"].as_array().unwrap();
    assert_eq!(items[0]["id"], 10);
    assert_eq!(items[1]["id"], 8);
    assert_eq!(items[2]["id"], 7);

    res = logged_in_client
        .post("/api/v1/plus/notifications/9/undo-deletion/", None, None)
        .await;
    assert_eq!(res.status(), 200);
    res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    res_json = read_json(res).await;
    let items = res_json["items"].as_array().unwrap();
    assert_eq!(items[0]["id"], 10);
    assert_eq!(items[1]["id"], 9);
    assert_eq!(items[2]["id"], 8);

    drop(stubr);
    Ok(())
}

#[actix_rt::test]
#[stubr::mock(port = 4321)]

async fn test_delete_many() -> Result<(), Error> {
    let pool = reset()?;
    wait_for_stubr().await?;
    let app = test_app_with_login(&pool).await?;
    let service = test::init_service(app).await;
    let mut logged_in_client = TestHttpClient::new(service).await;
    //Database was reset so we can naively assume user_id = 1.
    let _ids = create_notifications(&pool, 1, 10).await;

    let mut res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    let mut res_json = read_json(res).await;
    let mut items = res_json["items"].as_array().unwrap();
    assert_eq!(items.len(), 10);
    res = logged_in_client
        .post(
            "/api/v1/plus/notifications/delete-ids/",
            None,
            Some(PostPayload::Json(json!({"ids": [10,9,8]}))),
        )
        .await;

    assert_eq!(res.status(), 200);
    res = logged_in_client
        .get("/api/v1/plus/notifications/?offset=0&limit=10", None)
        .await;
    res_json = read_json(res).await;
    items = res_json["items"].as_array().unwrap();

    assert_eq!(items.len(), 7);
    assert_eq!(items[0]["id"], 7);
    assert_eq!(items[1]["id"], 6);
    assert_eq!(items[2]["id"], 5);

    drop(stubr);
    Ok(())
}

async fn create_notifications(pool: &Pool, user_id: i64, number: usize) -> Vec<i64> {
    let mut notification_ids: Vec<i64> = vec![];
    for i in 0..number {
        let uri = format!("{}/{}", "https://developer.allizom.org", i);
        let document = DocumentMetadata {
            parents: None,
            mdn_url: uri.to_string(),
            paths: vec![],
            title: format!("{} {}", "Test", i),
        };

        let document_id = db::documents::create_or_update_document(
            &mut pool.get().unwrap(),
            document,
            uri.to_string(),
        );

        let data = NotificationDataInsert {
            text: format!("Test text {}", i),
            url: uri.to_string(),
            data: None,
            title: format!("Test title {}", i),
            type_: if i % 2 == 0 {
                db::types::NotificationTypeEnum::Compat
            } else {
                db::types::NotificationTypeEnum::Content
            },
            document_id: document_id.unwrap(),
        };

        let notification_data_id =
            db::notifications::create_notification_data(&mut pool.get().unwrap(), data).unwrap();

        let id = db::notifications::create_notification(
            &mut pool.get().unwrap(),
            user_id,
            notification_data_id,
        )
        .unwrap();
        notification_ids.push(id);
    }
    notification_ids
}
