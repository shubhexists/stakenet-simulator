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

    pub fn new(epoch: u64, day: String, balance: BigDecimal) -> Self {
        Self {
            id: format!("{}-{}", epoch, day),
            epoch,
            day,
            balance,
        }
    }

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

    pub async fn fetch_balance_for_epoch_range(
        db_connection: &Pool<Postgres>,
        current_epoch: u64,
        lookback_period: u64,
    ) -> Result<BigDecimal, Error> {
        let start_epoch = current_epoch - lookback_period;
        let end_epoch = current_epoch;

        let query = "
            SELECT SUM(balance) as total_balance 
            FROM inactive_stake_jito_sol 
            WHERE epoch >= $1 AND epoch <= $2
        ";

        let result: Option<BigDecimal> = sqlx::query_scalar(query)
            .bind(BigDecimal::from(start_epoch))
            .bind(BigDecimal::from(end_epoch))
            .fetch_optional(db_connection)
            .await?;

        result.ok_or(Error::RowNotFound)
    }
}
