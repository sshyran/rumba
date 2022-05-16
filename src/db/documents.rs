use crate::db::model::{DocumentInsert, DocumentMetadata};
use crate::db::schema;

use diesel::{insert_into, PgConnection, QueryResult, RunQueryDsl};

use crate::settings::SETTINGS;

pub fn create_or_update_document(
    conn: &mut PgConnection,
    document: DocumentMetadata,
    uri: String,
) -> QueryResult<i64> {
    let absolute_uri = SETTINGS.application.document_base_url.clone() + &uri;

    let mut metadata = None;
    let title = document.title.clone();
    if let Ok(i) = serde_json::to_value(document) {
        metadata = Some(i);
    }

    let insert = DocumentInsert {
        title,
        absolute_uri,
        uri,
        metadata,
        updated_at: chrono::offset::Utc::now().naive_utc(),
    };

    insert_into(schema::documents::table)
        .values(&insert)
        .on_conflict(schema::documents::uri)
        .do_update()
        .set(&insert)
        .returning(schema::documents::id)
        .get_result(conn)
}
