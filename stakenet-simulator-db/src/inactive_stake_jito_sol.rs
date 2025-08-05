use crate::big_decimal_u64::BigDecimalU64;
use sqlx::{Error, FromRow, Pool, Postgres, QueryBuilder, types::BigDecimal};

#[derive(FromRow)]
pub struct InactiveStakeJitoSol {
    pub id: String,
    #[sqlx(try_from = "BigDecimalU64")]
    pub epoch: u64,
    pub day: String,
    pub balance: BigDecimal,
}

impl InactiveStakeJitoSol {
    const NUM_FIELDS: u8 = 4;
    // Based on the bind limit of postgres
    const INSERT_CHUNK_SIZE: usize = 65534 / Self::NUM_FIELDS as usize;
    const INSERT_QUERY: &str = "INSERT INTO inactive_stake_jito_sol (id,epoch,day,balance) VALUES ";

    pub async fn bulk_insert(
        db_connection: &Pool<Postgres>,
        records: Vec<Self>,
    ) -> Result<(), Error> {
        if records.len() <= 0 {
            return Ok(());
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(Self::INSERT_QUERY);
        let mut num_records: usize = 0;

        for record in records.into_iter() {
            num_records += 1;
            if num_records > 1 {
                query_builder.push(", (");
            } else {
                query_builder.push("(");
            }
            let mut separated = query_builder.separated(", ");
            separated.push_bind(record.id);
            separated.push_bind(BigDecimal::from(record.epoch));
            separated.push_bind(record.day);
            separated.push_bind(record.balance);
            separated.push_unseparated(") ");

            if num_records >= Self::INSERT_CHUNK_SIZE {
                query_builder.push(" ON CONFLICT (id) DO NOTHING");
                let query = query_builder.build();
                query.execute(db_connection).await?;
                num_records = 0;
                query_builder = QueryBuilder::new(Self::INSERT_QUERY);
            }
        }

        if num_records > 0 {
            query_builder.push(" ON CONFLICT (id) DO NOTHING");
            let query = query_builder.build();
            query.execute(db_connection).await?;
        }
        Ok(())
    }

    pub async fn fetch_by_epoch(
        db_connection: &Pool<Postgres>,
        epoch: u64,
    ) -> Result<Option<Self>, Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM inactive_stake_jito_sol WHERE epoch = $1")
            .bind(BigDecimal::from(epoch))
            .fetch_optional(db_connection)
            .await
    }

    pub async fn fetch_all_records(db_connection: &Pool<Postgres>) -> Result<Vec<Self>, Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM inactive_stake_jito_sol")
            .fetch_all(db_connection)
            .await
    }
}
