pub mod prisma;
use dotenvy::dotenv;
use prisma::PrismaClient;
use crate::error::{Error, ErrorKind};

pub async fn establish_connection() -> Result<PrismaClient, Error> {
    dotenv().ok();
    let res = prisma::new_client().await;
    
    match res {
        Ok(con) => {
            Ok(con)
        }
        Err(err) => {
            Err(Error{
                message: "Couldn't connect to the database".into(),
                action: "Try again later, or if you're the admin, check the database".into(),
                kind: ErrorKind::DatabaseError(err.to_string())
            })
        }
    }
}