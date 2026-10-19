use diesel::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{DisplayName, User, UserId};
use crate::{
    database::{self, DatabaseError, DatabasePool},
    schema::users,
};

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error("user record is malformed")]
    MalformedRecord,
}

#[derive(diesel::Queryable, diesel::Selectable)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
struct UserRow {
    id: Uuid,
    display_name: String,
    created_at: OffsetDateTime,
}

#[derive(diesel::Insertable)]
#[diesel(table_name = users)]
struct NewUserRow<'a> {
    id: Uuid,
    display_name: &'a str,
    created_at: OffsetDateTime,
}

impl TryFrom<UserRow> for User {
    type Error = UserError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: UserId::from(row.id),
            display_name: DisplayName::try_from(row.display_name)
                .map_err(|_| UserError::MalformedRecord)?,
            created_at: row.created_at,
        })
    }
}

/// # Errors
/// Fails if the database is unavailable or the stored record is malformed.
#[tracing::instrument(skip_all)]
pub async fn create(
    database: &DatabasePool,
    display_name: &DisplayName,
    now: OffsetDateTime,
) -> Result<User, UserError> {
    let row = database::within_timeout(async {
        let mut connection = database.get().await?;
        Ok(diesel::insert_into(users::table)
            .values(NewUserRow {
                id: UserId::generate().as_uuid(),
                display_name: display_name.as_str(),
                created_at: now,
            })
            .returning(UserRow::as_returning())
            .get_result(&mut connection)
            .await?)
    })
    .await?;
    User::try_from(row)
}

/// # Errors
/// Fails if the database is unavailable or the stored record is malformed.
#[tracing::instrument(skip_all, fields(user_id = %id))]
pub async fn find(database: &DatabasePool, id: UserId) -> Result<Option<User>, UserError> {
    database::within_timeout(async {
        let mut connection = database.get().await?;
        Ok(users::table
            .filter(users::id.eq(id.as_uuid()))
            .select(UserRow::as_select())
            .first(&mut connection)
            .await
            .optional()?)
    })
    .await?
    .map(User::try_from)
    .transpose()
}
