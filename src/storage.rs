use crate::Post;
use rjsdb::{DatabaseError, Database as Connection}

type Result<T> = std::result::Result<T, DatabaseError>;

pub struct Db {
    conn: Connection,
}
impl Db {
    pub fn build(conn: Connection) -> Result<Self> {
        let mut db = Db { conn };
        // TODO: Fix so that we're operating on a fresh db
        let version = db.get_version().unwrap_or(0);
        match version {
            0 => db.migrate_v0_v1().expect("Migrating version 0 to 1 failed"),
            1 => (),
            _ => panic!("Unknown db version found"),
        }
        let version = db.get_version().expect("Getting db version failed");

        assert_eq!(version, 1);

        Ok(db)
    }

    pub fn upsert_posts(
        &mut self,
        posts: impl Iterator<Item = Post>,
    ) -> Result<usize> {
        let mut tx = self.conn.transaction()?;
        let mut stmt = tx.prepare(
            "INSERT INTO posts(link, title, date, author) \
                            VALUES(:link, :title, :date, :author) \
                            ON CONFLICT(link) DO NOTHING;",
        )?;

        let mut rows_affected = 0;

        for post in posts {
            rows_affected += stmt.execute([
                (":link", &post.link),
                (":title", &post.title),
                (":date", &post.date.to_rfc3339()),
                (":author", &post.author),
            ].as_slice())?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(rows_affected)
    }

    pub fn fetch_all_posts(&self) -> Result<Vec<Post>> {
        let mut stmt = self
            .conn
            .prepare("SELECT link, title, date, author FROM posts ORDER BY date DESC;")?;

        let posts = stmt
            .query([])?
            .mapped(|row| {
                let d: String = row.get(2)?;
                let date = Post::parse_stored_date(&d).expect("Parsing stored date failed");
                let item = Post {
                    link: row.get(0)?,
                    title: row.get(1)?,
                    date,
                    author: row.get(3)?,
                };
                Ok(item)
            })
            .flatten()
            .collect();
        Ok(posts)
    }

    // MIGRATIONS
    fn get_version(&self) -> Result<usize> {
        self.conn
            .execute("CREATE TABLE IF NOT EXISTS _metadata(version INTEGER);")?; 

        let version: Option<usize> = self
            .conn
            .prepare("SELECT version FROM _metadata ORDER BY version DESC LIMIT 1;")?
            .query([])?
            .mapped([], |row| {
                let version: usize = row.get(0)?;
                Ok(version)
            })?
            .flatten()
            .next();

        Ok(version.unwrap_or(0))
    }

    fn migrate_v0_v1(&mut self) -> Result<()> {
        let mut tx = self.conn.transaction()?;
        // create table
        let rows_changed = tx.execute(
            "CREATE TABLE IF NOT EXISTS posts( \
                link TEXT PRIMARY KEY, \
                title TEXT, \
                date TEXT, \
                author TEXT \
            );",
        )?;
        assert_eq!(rows_changed, 0);

        tx.execute("INSERT INTO _metadata(version) VALUES(1)")?;
        tx.commit()?;

        Ok(())
    }
}

// pub fn get_db_version(conn: &Connection) -> Result<usize, rusqlite::Error> {}
