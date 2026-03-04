use crate::Post;
use turso::{Connection, Result};

pub struct Db {
    pub conn: Connection,
}
impl Db {
    pub async fn build(conn: Connection) -> Result<Self> {
        let mut db = Db { conn };

        // BEGIN Migration Transaction
        db.conn.execute("BEGIN;", ()).await?;
        let version = db
            .get_version()
            .await
            .expect("fetching initial version failed");
        match version {
            0 => db
                .migrate_v0_v1()
                .await
                .expect("Migrating version 0 to 1 failed"),
            1 => (),
            _ => panic!("Unknown db version found"),
        }
        let version = db.get_version().await.expect("Getting db version failed");
        db.conn.execute("COMMIT;", ()).await?;
        // END Migration Transaction

        assert_eq!(version, 1);

        Ok(db)
    }

    pub async fn upsert_posts(&mut self, posts: impl Iterator<Item = Post>) -> Result<u64> {
        // try 3 times
        self.conn.execute("BEGIN;", ()).await?;
        let mut stmt = self
            .conn
            .prepare(
                "INSERT INTO posts(link, title, date, author) \
                            VALUES(?1, ?2, ?3, ?4) \
                            ON CONFLICT(link) DO NOTHING;",
            )
            .await?;

        let mut rows_affected = 0;

        for post in posts {
            rows_affected += stmt
                .execute([post.link, post.title, post.date.to_rfc3339(), post.author])
                .await?;
        }
        drop(stmt);
        self.conn.execute("COMMIT;", ()).await?;
        Ok(rows_affected)
    }

    pub async fn fetch_all_posts(&mut self) -> Result<Vec<Post>> {
        let mut stmt = self
            .conn
            .prepare("SELECT link, title, date, author FROM posts ORDER BY date DESC;")
            .await?;

        let mut results = stmt.query(()).await?;
        let mut posts = Vec::new();
        while let Some(row) = results.next().await? {
            let date_value = row.get_value(2)?;
            let date_str = date_value.as_text().expect("Failed to get stored date");
            let date = Post::parse_stored_date(date_str).expect("Parsing stored date failed");
            let item = Post {
                link: row
                    .get_value(0)?
                    .as_text()
                    .expect("Failed to get stored link")
                    .to_string(),
                title: row
                    .get_value(1)?
                    .as_text()
                    .expect("Failed to get stored title")
                    .to_string(),
                date,
                author: row
                    .get_value(3)?
                    .as_text()
                    .expect("Failed to get stored author")
                    .to_string(),
            };
            posts.push(item);
        }
        Ok(posts)
    }

    // MIGRATIONS
    async fn get_version(&mut self) -> Result<i64> {
        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS _metadata(version UNSIGNED INT);",
                (),
            )
            .await?;

        let mut rows = self
            .conn
            .prepare("SELECT version FROM _metadata ORDER BY version DESC LIMIT 1;")
            .await?
            .query(())
            .await?;
        let version = rows.next().await?.and_then(|r| {
            r.get_value(0)
                .expect("Failed to get db version number when it should exist")
                .as_integer()
                .cloned()
        });

        Ok(version.unwrap_or(0))
    }

    async fn migrate_v0_v1(&mut self) -> Result<()> {
        // create table
        let rows_changed = self
            .conn
            .execute(
                "CREATE TABLE IF NOT EXISTS posts( \
                link STRING PRIMARY KEY, \
                title STRING, \
                date STRING, \
                author STRING \
            );",
                (),
            )
            .await?;
        assert_eq!(rows_changed, 0);

        self.conn
            .execute("INSERT INTO _metadata(version) VALUES(1);", ())
            .await?;

        Ok(())
    }
}

// pub fn get_db_version(conn: &Connection) -> Result<usize, rusqlite::Error> {}
