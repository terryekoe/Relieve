use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptureVerse {
    pub book: String,
    pub chapter: u32,
    pub verse: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScripturePassage {
    pub reference: String,
    pub translation: String,
    pub book: String,
    pub chapter: u32,
    pub verse_start: u32,
    pub verse_end: Option<u32>,
    pub verses: Vec<ScriptureVerse>,
    pub combined_text: String,
}

pub struct BibleDatabase {
    conn: Mutex<Connection>,
}

impl BibleDatabase {
    pub fn new_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_tables()?;
        db.seed_initial_verses()?;
        Ok(db)
    }

    pub fn new_at_path(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_tables()?;
        db.seed_initial_verses()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS verses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                translation TEXT NOT NULL,
                book TEXT NOT NULL,
                chapter INTEGER NOT NULL,
                verse INTEGER NOT NULL,
                text TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_bcv ON verses(translation, book, chapter, verse);
            ",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn seed_initial_verses(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM verses", [], |r| r.get(0))
            .unwrap_or(0);
        if count > 0 {
            return Ok(());
        }

        let mut stmt = conn
            .prepare("INSERT INTO verses (translation, book, chapter, verse, text) VALUES (?1, ?2, ?3, ?4, ?5)")
            .map_err(|e| e.to_string())?;

        let sample_verses = [
            // John 3:16-17
            ("KJV", "John", 3, 16, "For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life."),
            ("KJV", "John", 3, 17, "For God sent not his Son into the world to condemn the world; but that the world through him might be saved."),
            ("WEB", "John", 3, 16, "For God so loved the world, that he gave his one and only Son, that whoever believes in him should not perish, but have eternal life."),
            ("WEB", "John", 3, 17, "For God didn't send his Son into the world to judge the world, but that the world should be saved through him."),

            // Romans 8:28-31
            ("KJV", "Romans", 8, 28, "And we know that all things work together for good to them that love God, to them who are the called according to his purpose."),
            ("KJV", "Romans", 8, 29, "For whom he did foreknow, he also did predestinate to be conformed to the image of his Son, that he might be the firstborn among many brethren."),
            ("KJV", "Romans", 8, 30, "Moreover whom he did predestinate, them he also called: and whom he called, them he also justified: and whom he justified, them he also glorified."),
            ("KJV", "Romans", 8, 31, "What shall we then say to these things? If God be for us, who can be against us?"),
            ("WEB", "Romans", 8, 28, "We know that all things work together for good for those who love God, to those who are called according to his purpose."),
            ("WEB", "Romans", 8, 29, "For whom he foreknew, he also predestined to be conformed to the image of his Son, that he might be the firstborn among many brothers."),
            ("WEB", "Romans", 8, 30, "Whom he predestined, those he also called. Whom he called, those he also justified. Whom he justified, those he also glorified."),
            ("WEB", "Romans", 8, 31, "What then shall we say about these things? If God is for us, who can be against us?"),

            // Psalm 23:1-6
            ("KJV", "Psalms", 23, 1, "The LORD is my shepherd; I shall not want."),
            ("KJV", "Psalms", 23, 2, "He maketh me to lie down in green pastures: he leadeth me beside the still waters."),
            ("KJV", "Psalms", 23, 3, "He restoreth my soul: he leadeth me in the paths of righteousness for his name's sake."),
            ("KJV", "Psalms", 23, 4, "Yea, though I walk through the valley of the shadow of death, I will fear no evil: for thou art with me; thy rod and thy staff they comfort me."),
            ("KJV", "Psalms", 23, 5, "Thou preparest a table before me in the presence of mine enemies: thou anointest my head with oil; my cup runneth over."),
            ("KJV", "Psalms", 23, 6, "Surely goodness and mercy shall follow me all the days of my life: and I will dwell in the house of the LORD for ever."),

            // 1 Corinthians 13:4-8
            ("KJV", "1 Corinthians", 13, 4, "Charity suffereth long, and is kind; charity envieth not; charity vaunteth not itself, is not puffed up,"),
            ("KJV", "1 Corinthians", 13, 5, "Doth not behave itself unseemly, seeketh not her own, is not easily provoked, thinketh no evil;"),
            ("KJV", "1 Corinthians", 13, 6, "Rejoiceth not in iniquity, but rejoiceth in the truth;"),
            ("KJV", "1 Corinthians", 13, 7, "Beareth all things, believeth all things, hopeth all things, endureth all things."),
            ("KJV", "1 Corinthians", 13, 8, "Charity never faileth: but whether there be prophecies, they shall fail; whether there be tongues, they shall cease; whether there be knowledge, it shall vanish away."),

            // Genesis 1:1-3
            ("KJV", "Genesis", 1, 1, "In the beginning God created the heaven and the earth."),
            ("KJV", "Genesis", 1, 2, "And the earth was without form, and void; and darkness was upon the face of the deep. And the Spirit of God moved upon the face of the waters."),
            ("KJV", "Genesis", 1, 3, "And God said, Let there be light: and there was light."),

            // Philippians 4:13
            ("KJV", "Philippians", 4, 13, "I can do all things through Christ which strengtheneth me."),
            ("WEB", "Philippians", 4, 13, "I can do all things through Christ, who strengthens me."),

            // Hebrews 11:1
            ("KJV", "Hebrews", 11, 1, "Now faith is the substance of things hoped for, the evidence of things not seen."),
            ("WEB", "Hebrews", 11, 1, "Now faith is the assurance of things hoped for, proof of things not seen."),
        ];

        for (trans, book, ch, v, text) in sample_verses {
            stmt.execute(params![trans, book, ch, v, text])
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub fn lookup_passage(
        &self,
        translation: &str,
        book: &str,
        chapter: u32,
        verse_start: u32,
        verse_end: Option<u32>,
    ) -> Result<ScripturePassage, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let v_end = verse_end.unwrap_or(verse_start);

        let mut stmt = conn
            .prepare(
                "SELECT book, chapter, verse, text FROM verses 
                 WHERE translation = ?1 AND book = ?2 AND chapter = ?3 AND verse >= ?4 AND verse <= ?5 
                 ORDER BY verse ASC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![translation, book, chapter, verse_start, v_end], |row| {
                Ok(ScriptureVerse {
                    book: row.get(0)?,
                    chapter: row.get(1)?,
                    verse: row.get(2)?,
                    text: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut verses = Vec::new();
        for r in rows {
            verses.push(r.map_err(|e| e.to_string())?);
        }

        if verses.is_empty() {
            // If the specific translation has no match, attempt fallback to KJV
            if translation != "KJV" {
                drop(stmt);
                drop(conn);
                return self.lookup_passage("KJV", book, chapter, verse_start, verse_end);
            }
            return Err(format!(
                "Passage not found: {} {}:{}{}",
                book,
                chapter,
                verse_start,
                verse_end.map(|e| format!("-{}", e)).unwrap_or_default()
            ));
        }

        let ref_str = if let Some(end) = verse_end {
            if end != verse_start {
                format!("{} {}:{}-{}", book, chapter, verse_start, end)
            } else {
                format!("{} {}:{}", book, chapter, verse_start)
            }
        } else {
            format!("{} {}:{}", book, chapter, verse_start)
        };

        let combined = verses
            .iter()
            .map(|v| v.text.clone())
            .collect::<Vec<_>>()
            .join(" ");

        Ok(ScripturePassage {
            reference: ref_str,
            translation: translation.to_string(),
            book: book.to_string(),
            chapter,
            verse_start,
            verse_end,
            verses,
            combined_text: combined,
        })
    }

    pub fn search_verses(
        &self,
        translation: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ScriptureVerse>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let pattern = format!("%{}%", query);

        let mut stmt = conn
            .prepare(
                "SELECT book, chapter, verse, text FROM verses 
                 WHERE translation = ?1 AND text LIKE ?2 
                 LIMIT ?3",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![translation, pattern, limit as i64], |row| {
                Ok(ScriptureVerse {
                    book: row.get(0)?,
                    chapter: row.get(1)?,
                    verse: row.get(2)?,
                    text: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r.map_err(|e| e.to_string())?);
        }

        Ok(results)
    }

    /// Matches spoken text directly against verse content in the Bible database.
    /// Used when a speaker recites scripture directly without explicitly stating the book/chapter/verse.
    /// (e.g. "For God so loved the world that he gave his only begotten Son")
    pub fn match_spoken_verse_text(
        &self,
        translation: &str,
        spoken_text: &str,
    ) -> Result<Option<ScripturePassage>, String> {
        let cleaned = spoken_text.to_lowercase();
        // Extract distinct words with length >= 4 (filtering out generic stop words)
        let stop_words: std::collections::HashSet<&str> = [
            "that", "this", "with", "from", "they", "will", "have", "been", "were",
            "what", "when", "your", "them", "then", "into", "also", "there", "their",
            "unto", "said", "which", "shall", "upon", "come", "came", "shalt",
        ].into_iter().collect();

        let mut words: Vec<&str> = cleaned
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| w.len() >= 4 && !stop_words.contains(w))
            .collect();

        // Need at least 3 significant words to search safely
        if words.len() < 3 {
            return Ok(None);
        }

        // Deduplicate words
        words.sort_unstable();
        words.dedup();
        // Sort by length descending to pick the most distinctive words
        words.sort_by(|a, b| b.len().cmp(&a.len()));

        // Use the top 2 longest words for initial candidate query
        let top_words = &words[..words.len().min(2)];
        let pattern1 = format!("%{}%", top_words[0]);
        let pattern2 = if top_words.len() > 1 {
            format!("%{}%", top_words[1])
        } else {
            pattern1.clone()
        };

        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT book, chapter, verse, text FROM verses 
                 WHERE (translation = ?1 OR translation = 'KJV') 
                   AND text LIKE ?2 AND text LIKE ?3 
                 LIMIT 10",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![translation, pattern1, pattern2], |row| {
                Ok(ScriptureVerse {
                    book: row.get(0)?,
                    chapter: row.get(1)?,
                    verse: row.get(2)?,
                    text: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut best_match: Option<ScriptureVerse> = None;
        let mut highest_score = 0.0f32;

        let spoken_words_set: std::collections::HashSet<&str> = cleaned.split_whitespace().collect();

        for r in rows {
            if let Ok(v) = r {
                let verse_lower = v.text.to_lowercase();
                let verse_words: Vec<&str> = verse_lower
                    .split_whitespace()
                    .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
                    .filter(|w| !w.is_empty())
                    .collect();

                if verse_words.is_empty() {
                    continue;
                }

                // Count token matches
                let mut matched_count = 0;
                for vw in &verse_words {
                    if spoken_words_set.contains(vw) {
                        matched_count += 1;
                    }
                }

                let score = matched_count as f32 / verse_words.len().min(spoken_words_set.len()) as f32;
                // High confidence threshold (>= 0.55 overlap and >= 4 words matched)
                if matched_count >= 4 && score >= 0.55 && score > highest_score {
                    highest_score = score;
                    best_match = Some(v);
                }
            }
        }

        if let Some(v) = best_match {
            drop(stmt);
            drop(conn);
            return self.lookup_passage(translation, &v.book, v.chapter, v.verse, None).map(Some);
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_john_3_16() {
        let db = BibleDatabase::new_in_memory().unwrap();
        let passage = db.lookup_passage("KJV", "John", 3, 16, None).unwrap();
        assert_eq!(passage.reference, "John 3:16");
        assert!(passage.combined_text.contains("For God so loved the world"));
    }

    #[test]
    fn test_lookup_range_romans_8() {
        let db = BibleDatabase::new_in_memory().unwrap();
        let passage = db.lookup_passage("KJV", "Romans", 8, 28, Some(30)).unwrap();
        assert_eq!(passage.reference, "Romans 8:28-30");
        assert_eq!(passage.verses.len(), 3);
        assert_eq!(passage.verses[0].verse, 28);
        assert_eq!(passage.verses[2].verse, 30);
    }

    #[test]
    fn test_match_spoken_verse_text() {
        let db = BibleDatabase::new_in_memory().unwrap();
        // Direct quotation of John 3:16
        let spoken = "for God so loved the world that he gave his only begotten Son";
        let matched = db.match_spoken_verse_text("KJV", spoken).unwrap().expect("Should match John 3:16 text");
        assert_eq!(matched.reference, "John 3:16");

        // Direct quotation of Psalm 23:1
        let spoken2 = "the Lord is my shepherd I shall not want";
        let matched2 = db.match_spoken_verse_text("KJV", spoken2).unwrap().expect("Should match Psalm 23:1 text");
        assert_eq!(matched2.reference, "Psalms 23:1");
    }
}
