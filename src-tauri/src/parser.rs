use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use strsim::{jaro_winkler, levenshtein as strsim_levenshtein};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpokenCitation {
    pub book: String,
    pub chapter: u32,
    pub verse_start: u32,
    pub verse_end: Option<u32>,
    pub confidence: f32,
    pub raw_match: String,
}

// Canonical list of all 66 Bible books
pub static CANONICAL_BOOKS: [&str; 66] = [
    "Genesis", "Exodus", "Leviticus", "Numbers", "Deuteronomy", "Joshua", "Judges", "Ruth",
    "1 Samuel", "2 Samuel", "1 Kings", "2 Kings", "1 Chronicles", "2 Chronicles", "Ezra",
    "Nehemiah", "Esther", "Job", "Psalms", "Proverbs", "Ecclesiastes", "Song of Solomon",
    "Isaiah", "Jeremiah", "Lamentations", "Ezekiel", "Daniel", "Hosea", "Joel", "Amos",
    "Obadiah", "Jonah", "Micah", "Nahum", "Habakkuk", "Zephaniah", "Haggai", "Zechariah",
    "Malachi", "Matthew", "Mark", "Luke", "John", "Acts", "Romans", "1 Corinthians",
    "2 Corinthians", "Galatians", "Ephesians", "Philippians", "Colossians", "1 Thessalonians",
    "2 Thessalonians", "1 Timothy", "2 Timothy", "Titus", "Philemon", "Hebrews", "James",
    "1 Peter", "2 Peter", "1 John", "2 John", "3 John", "Jude", "Revelation",
];

/// Common English words that must never be fuzzy matched to Bible books in shorthand mode
static STOPWORDS_BLACKLIST: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    for word in [
        "join", "make", "take", "truth", "good", "book", "look", "come", "some", "time",
        "have", "with", "from", "said", "they", "will", "what", "about", "the", "and",
        "that", "this", "here", "just", "like", "when", "then", "into", "more", "also",
        "over", "only", "even", "back", "well", "down", "much", "very", "your", "them",
        "give", "tell", "read", "know", "hear", "word", "lord", "turn", "open", "bible",
    ] {
        s.insert(word);
    }
    s
});

/// Computes Levenshtein edit distance between two strings
pub fn levenshtein(a: &str, b: &str) -> usize {
    strsim_levenshtein(a, b)
}

// Canonical list of standard 66 Bible books and spoken / phonetic aliases
static BOOK_ALIASES: Lazy<HashMap<String, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    
    // Automatically insert all canonical books (lowercased)
    for &book in CANONICAL_BOOKS.iter() {
        m.insert(book.to_lowercase(), book);
    }

    let mappings = [
        // ==========================================
        // OLD TESTAMENT (39 Books)
        // ==========================================
        // Genesis
        ("genesis", "Genesis"), ("gen", "Genesis"), ("genisis", "Genesis"),
        ("jenesis", "Genesis"), ("djenesis", "Genesis"), ("genese", "Genesis"),

        // Exodus
        ("exodus", "Exodus"), ("exo", "Exodus"), ("exodous", "Exodus"),
        ("eksodus", "Exodus"), ("exodos", "Exodus"), ("eksodos", "Exodus"),

        // Leviticus
        ("leviticus", "Leviticus"), ("lev", "Leviticus"), ("levitikus", "Leviticus"),
        ("leviticos", "Leviticus"), ("lebiticus", "Leviticus"), ("libiticus", "Leviticus"),

        // Numbers
        ("numbers", "Numbers"), ("num", "Numbers"), ("numbas", "Numbers"),
        ("nombas", "Numbers"), ("number", "Numbers"), ("nomba", "Numbers"),

        // Deuteronomy
        ("deuteronomy", "Deuteronomy"), ("deut", "Deuteronomy"), ("deutronomy", "Deuteronomy"),
        ("duteronomy", "Deuteronomy"), ("dutronomy", "Deuteronomy"), ("diutronomy", "Deuteronomy"),

        // Joshua
        ("joshua", "Joshua"), ("josh", "Joshua"), ("yoshua", "Joshua"), ("joshuwa", "Joshua"),

        // Judges
        ("judges", "Judges"), ("judg", "Judges"), ("jadjis", "Judges"),
        ("jadges", "Judges"), ("judjis", "Judges"), ("juges", "Judges"),

        // Ruth
        ("ruth", "Ruth"), ("rut", "Ruth"), ("rooth", "Ruth"), ("root", "Ruth"),

        // 1 Samuel
        ("1 samuel", "1 Samuel"), ("1 sam", "1 Samuel"), ("1st samuel", "1 Samuel"), ("1st sam", "1 Samuel"),
        ("first samuel", "1 Samuel"), ("first sam", "1 Samuel"),
        ("fess samuel", "1 Samuel"), ("fess sam", "1 Samuel"),
        ("fes samuel", "1 Samuel"), ("fes sam", "1 Samuel"),
        ("fist samuel", "1 Samuel"), ("fist sam", "1 Samuel"),
        ("fest samuel", "1 Samuel"), ("fas samuel", "1 Samuel"),
        ("samuel 1", "1 Samuel"), ("samwel 1", "1 Samuel"), ("1 samwel", "1 Samuel"), ("first samwel", "1 Samuel"),

        // 2 Samuel
        ("2 samuel", "2 Samuel"), ("2 sam", "2 Samuel"), ("2nd samuel", "2 Samuel"), ("2nd sam", "2 Samuel"),
        ("second samuel", "2 Samuel"), ("second sam", "2 Samuel"),
        ("seken samuel", "2 Samuel"), ("seken sam", "2 Samuel"),
        ("sekond samuel", "2 Samuel"), ("sekond sam", "2 Samuel"),
        ("seknd samuel", "2 Samuel"), ("sekken samuel", "2 Samuel"),
        // Single-mention Samuel (defaults to 1 Samuel)
        ("samuel", "1 Samuel"), ("samwel", "1 Samuel"),

        // 1 Kings
        ("1 kings", "1 Kings"), ("1 kgs", "1 Kings"), ("1st kings", "1 Kings"),
        ("first kings", "1 Kings"), ("fess kings", "1 Kings"), ("fes kings", "1 Kings"),
        ("fist kings", "1 Kings"), ("fest kings", "1 Kings"), ("kings 1", "1 Kings"),

        // 2 Kings
        ("2 kings", "2 Kings"), ("2 kgs", "2 Kings"), ("2nd kings", "2 Kings"),
        ("second kings", "2 Kings"), ("seken kings", "2 Kings"), ("sekond kings", "2 Kings"),
        ("seknd kings", "2 Kings"), ("kings 2", "2 Kings"),

        // Single-mention Kings (defaults to 1 Kings)
        ("kings", "1 Kings"), ("kgs", "1 Kings"),

        // 1 Chronicles
        ("1 chronicles", "1 Chronicles"), ("1 chron", "1 Chronicles"), ("1st chronicles", "1 Chronicles"),
        ("first chronicles", "1 Chronicles"), ("fess chronicles", "1 Chronicles"), ("fes chronicles", "1 Chronicles"),
        ("first kronikles", "1 Chronicles"), ("first kronicles", "1 Chronicles"),
        ("first koronikles", "1 Chronicles"), ("first korunikles", "1 Chronicles"),
        ("fess kronikles", "1 Chronicles"), ("1 kronikles", "1 Chronicles"),

        // 2 Chronicles
        ("2 chronicles", "2 Chronicles"), ("2 chron", "2 Chronicles"), ("2nd chronicles", "2 Chronicles"),
        ("second chronicles", "2 Chronicles"), ("seken chronicles", "2 Chronicles"), ("sekond chronicles", "2 Chronicles"),
        ("second kronikles", "2 Chronicles"), ("second kronicles", "2 Chronicles"),
        ("second koronikles", "2 Chronicles"), ("second korunikles", "2 Chronicles"),
        ("seken kronikles", "2 Chronicles"), ("2 kronikles", "2 Chronicles"),

        // Single-mention Chronicles (defaults to 1 Chronicles)
        ("chronicles", "1 Chronicles"), ("chron", "1 Chronicles"), ("kronikles", "1 Chronicles"),

        // Ezra
        ("ezra", "Ezra"), ("ezr", "Ezra"), ("ezrah", "Ezra"),

        // Nehemiah
        ("nehemiah", "Nehemiah"), ("neh", "Nehemiah"), ("nehemia", "Nehemiah"), ("nehemaya", "Nehemiah"),

        // Esther
        ("esther", "Esther"), ("est", "Esther"), ("esta", "Esther"), ("ester", "Esther"), ("hesta", "Esther"),

        // Job
        ("job", "Job"), ("jobe", "Job"), ("djob", "Job"),

        // Psalms
        ("psalms", "Psalms"), ("psalm", "Psalms"), ("psa", "Psalms"),
        ("sam", "Psalms"), ("sams", "Psalms"), ("salm", "Psalms"), ("salms", "Psalms"),
        ("pslam", "Psalms"), ("pslams", "Psalms"), ("sohm", "Psalms"), ("sahm", "Psalms"),

        // Proverbs
        ("proverbs", "Proverbs"), ("proverb", "Proverbs"), ("prov", "Proverbs"),
        ("pro", "Proverbs"), ("provebs", "Proverbs"), ("ploverbs", "Proverbs"),

        // Ecclesiastes
        ("ecclesiastes", "Ecclesiastes"), ("ecc", "Ecclesiastes"), ("ecclesiastis", "Ecclesiastes"),
        ("eklesiastes", "Ecclesiastes"), ("eklizyastis", "Ecclesiastes"), ("eclesiastes", "Ecclesiastes"),

        // Song of Solomon
        ("song of solomon", "Song of Solomon"), ("song of songs", "Song of Solomon"),
        ("canticles", "Song of Solomon"), ("solomon song", "Song of Solomon"), ("sos", "Song of Solomon"),

        // Isaiah
        ("isaiah", "Isaiah"), ("isa", "Isaiah"), ("isaya", "Isaiah"),
        ("aizaya", "Isaiah"), ("aisayah", "Isaiah"), ("isaia", "Isaiah"),

        // Jeremiah
        ("jeremiah", "Jeremiah"), ("jer", "Jeremiah"), ("jeremia", "Jeremiah"),
        ("jeremaya", "Jeremiah"), ("djeremiah", "Jeremiah"),

        // Lamentations
        ("lamentations", "Lamentations"), ("lam", "Lamentations"), ("lamentation", "Lamentations"),
        ("lamentasion", "Lamentations"), ("lamentashuns", "Lamentations"),

        // Ezekiel
        ("ezekiel", "Ezekiel"), ("ezek", "Ezekiel"), ("ezikiel", "Ezekiel"), ("ezekyel", "Ezekiel"),

        // Daniel
        ("daniel", "Daniel"), ("dan", "Daniel"), ("danyel", "Daniel"), ("danier", "Daniel"),

        // Hosea
        ("hosea", "Hosea"), ("hos", "Hosea"), ("hoseya", "Hosea"), ("osiya", "Hosea"),

        // Joel
        ("joel", "Joel"), ("joe", "Joel"), ("jowell", "Joel"), ("djoel", "Joel"),

        // Amos
        ("amos", "Amos"), ("amo", "Amos"), ("aymos", "Amos"),

        // Obadiah
        ("obadiah", "Obadiah"), ("oba", "Obadiah"), ("obadaya", "Obadiah"), ("obadia", "Obadiah"),

        // Jonah
        ("jonah", "Jonah"), ("jon", "Jonah"), ("jona", "Jonah"), ("djona", "Jonah"),

        // Micah
        ("micah", "Micah"), ("mic", "Micah"), ("mika", "Micah"), ("maika", "Micah"), ("micha", "Micah"),

        // Nahum
        ("nahum", "Nahum"), ("nah", "Nahum"), ("nayhum", "Nahum"),

        // Habakkuk
        ("habakkuk", "Habakkuk"), ("hab", "Habakkuk"), ("habakuk", "Habakkuk"),
        ("habakook", "Habakkuk"), ("habacock", "Habakkuk"),

        // Zephaniah
        ("zephaniah", "Zephaniah"), ("zep", "Zephaniah"), ("zefanaya", "Zephaniah"),
        ("zepaniah", "Zephaniah"), ("zefaniah", "Zephaniah"),

        // Haggai
        ("haggai", "Haggai"), ("hag", "Haggai"), ("hagai", "Haggai"), ("haggay", "Haggai"),

        // Zechariah
        ("zechariah", "Zechariah"), ("zech", "Zechariah"), ("zekaraya", "Zechariah"),
        ("zekaria", "Zechariah"), ("zakariya", "Zechariah"),

        // Malachi
        ("malachi", "Malachi"), ("mal", "Malachi"), ("malakai", "Malachi"),
        ("malaki", "Malachi"), ("malachee", "Malachi"),

        // ==========================================
        // NEW TESTAMENT (27 Books)
        // ==========================================
        // Matthew
        ("matthew", "Matthew"), ("matt", "Matthew"), ("mathew", "Matthew"),
        ("matiu", "Matthew"), ("matew", "Matthew"),

        // Mark
        ("mark", "Mark"), ("mrk", "Mark"), ("mak", "Mark"), ("maak", "Mark"),

        // Luke
        ("luke", "Luke"), ("luk", "Luke"), ("look", "Luke"),

        // John
        ("john", "John"), ("jhn", "John"), ("jn", "John"),
        ("jon", "John"), ("jahn", "John"), ("djohn", "John"),

        // Acts
        ("acts", "Acts"), ("act", "Acts"), ("ax", "Acts"), ("akts", "Acts"),
        ("acts of the apostles", "Acts"),

        // Romans
        ("romans", "Romans"), ("rom", "Romans"), ("roman", "Romans"),
        ("romens", "Romans"), ("romance", "Romans"),

        // 1 Corinthians
        ("1 corinthians", "1 Corinthians"), ("1 cor", "1 Corinthians"), ("1st corinthians", "1 Corinthians"),
        ("first corinthians", "1 Corinthians"), ("first cor", "1 Corinthians"),
        ("fess corinthians", "1 Corinthians"), ("fes corinthians", "1 Corinthians"),
        ("fist corinthians", "1 Corinthians"), ("first korinthians", "1 Corinthians"),
        ("first korinteans", "1 Corinthians"), ("first corinteans", "1 Corinthians"),
        ("first colinthians", "1 Corinthians"), ("1 colinthians", "1 Corinthians"),
        ("corinthians 1", "1 Corinthians"),

        // 2 Corinthians
        ("2 corinthians", "2 Corinthians"), ("2 cor", "2 Corinthians"), ("2nd corinthians", "2 Corinthians"),
        ("second corinthians", "2 Corinthians"), ("second cor", "2 Corinthians"),
        ("seken corinthians", "2 Corinthians"), ("sekond corinthians", "2 Corinthians"),
        ("second korinthians", "2 Corinthians"), ("second korinteans", "2 Corinthians"),
        ("second colinthians", "2 Corinthians"), ("2 colinthians", "2 Corinthians"),
        ("corinthians 2", "2 Corinthians"),

        // Single-mention Corinthians (defaults to 1 Corinthians if speaker omitted prefix)
        ("corinthians", "1 Corinthians"), ("corinthian", "1 Corinthians"),
        ("korinthians", "1 Corinthians"), ("korinteans", "1 Corinthians"),
        ("colinthians", "1 Corinthians"), ("colinthian", "1 Corinthians"),

        // Galatians
        ("galatians", "Galatians"), ("gal", "Galatians"), ("galatian", "Galatians"),
        ("galashians", "Galatians"), ("galatia", "Galatians"), ("galasian", "Galatians"),

        // Ephesians
        ("ephesians", "Ephesians"), ("eph", "Ephesians"), ("ephesian", "Ephesians"),
        ("efisians", "Ephesians"), ("efesians", "Ephesians"),

        // Philippians
        ("philippians", "Philippians"), ("phil", "Philippians"), ("philipians", "Philippians"),
        ("philippian", "Philippians"), ("philipian", "Philippians"), ("filipians", "Philippians"),
        ("filipian", "Philippians"),

        // Colossians
        ("colossians", "Colossians"), ("col", "Colossians"), ("colossian", "Colossians"),
        ("colosians", "Colossians"), ("kolosians", "Colossians"), ("koloshians", "Colossians"),

        // 1 Thessalonians
        ("1 thessalonians", "1 Thessalonians"), ("1 thess", "1 Thessalonians"), ("1st thessalonians", "1 Thessalonians"),
        ("first thessalonians", "1 Thessalonians"), ("first thess", "1 Thessalonians"),
        ("fess thessalonians", "1 Thessalonians"), ("first tesalonians", "1 Thessalonians"),
        ("first thesalonians", "1 Thessalonians"), ("fess thess", "1 Thessalonians"),
        ("thessalonians 1", "1 Thessalonians"),

        // 2 Thessalonians
        ("2 thessalonians", "2 Thessalonians"), ("2 thess", "2 Thessalonians"), ("2nd thessalonians", "2 Thessalonians"),
        ("second thessalonians", "2 Thessalonians"), ("second thess", "2 Thessalonians"),
        ("seken thessalonians", "2 Thessalonians"), ("second tesalonians", "2 Thessalonians"),
        ("second thesalonians", "2 Thessalonians"), ("seken thess", "2 Thessalonians"),
        ("thessalonians 2", "2 Thessalonians"),

        // Single-mention Thessalonians (defaults to 1 Thessalonians)
        ("thessalonians", "1 Thessalonians"), ("thessalonian", "1 Thessalonians"),
        ("tesalonians", "1 Thessalonians"),

        // 1 Timothy
        ("1 timothy", "1 Timothy"), ("1 tim", "1 Timothy"), ("1st timothy", "1 Timothy"),
        ("first timothy", "1 Timothy"), ("first tim", "1 Timothy"),
        ("fess timothy", "1 Timothy"), ("fess tim", "1 Timothy"),
        ("first timoty", "1 Timothy"), ("timothy 1", "1 Timothy"),

        // 2 Timothy
        ("2 timothy", "2 Timothy"), ("2 tim", "2 Timothy"), ("2nd timothy", "2 Timothy"),
        ("second timothy", "2 Timothy"), ("second tim", "2 Timothy"),
        ("seken timothy", "2 Timothy"), ("seken tim", "2 Timothy"),
        ("second timoty", "2 Timothy"), ("timothy 2", "2 Timothy"),

        // Single-mention Timothy (defaults to 1 Timothy)
        ("timothy", "1 Timothy"), ("timoty", "1 Timothy"), ("timoti", "1 Timothy"),

        // Titus
        ("titus", "Titus"), ("tit", "Titus"), ("taiturs", "Titus"), ("titos", "Titus"), ("taitus", "Titus"),

        // Philemon
        ("philemon", "Philemon"), ("phm", "Philemon"), ("filemon", "Philemon"), ("filimon", "Philemon"),

        // Hebrews
        ("hebrews", "Hebrews"), ("heb", "Hebrews"), ("hebrew", "Hebrews"), ("hebru", "Hebrews"),

        // James
        ("james", "James"), ("jas", "James"), ("jemz", "James"), ("jeims", "James"), ("jems", "James"),

        // 1 Peter
        ("1 peter", "1 Peter"), ("1 pet", "1 Peter"), ("1st peter", "1 Peter"),
        ("first peter", "1 Peter"), ("first pet", "1 Peter"),
        ("fess peter", "1 Peter"), ("fess pet", "1 Peter"),
        ("first pita", "1 Peter"), ("fess pita", "1 Peter"),
        ("peter 1", "1 Peter"),

        // 2 Peter
        ("2 peter", "2 Peter"), ("2 pet", "2 Peter"), ("2nd peter", "2 Peter"),
        ("second peter", "2 Peter"), ("second pet", "2 Peter"),
        ("seken peter", "2 Peter"), ("seken pet", "2 Peter"),
        ("second pita", "2 Peter"), ("peter 2", "2 Peter"),

        // Single-mention Peter (defaults to 1 Peter)
        ("peter", "1 Peter"), ("pita", "1 Peter"),

        // 1 John
        ("1 john", "1 John"), ("1 jhn", "1 John"), ("1st john", "1 John"),
        ("first john", "1 John"), ("fess john", "1 John"), ("fes john", "1 John"),
        ("first jon", "1 John"), ("fess jon", "1 John"), ("john 1", "1 John"),

        // 2 John
        ("2 john", "2 John"), ("2 jhn", "2 John"), ("2nd john", "2 John"),
        ("second john", "2 John"), ("seken john", "2 John"),
        ("second jon", "2 John"), ("seken jon", "2 John"), ("john 2", "2 John"),

        // 3 John
        ("3 john", "3 John"), ("3 jhn", "3 John"), ("3rd john", "3 John"),
        ("third john", "3 John"), ("tud john", "3 John"), ("toid john", "3 John"),
        ("tird john", "3 John"), ("ted john", "3 John"), ("john 3", "3 John"),

        // Jude
        ("jude", "Jude"), ("jud", "Jude"), ("djud", "Jude"), ("jood", "Jude"),

        // Revelation
        ("revelation", "Revelation"), ("revelations", "Revelation"), ("rev", "Revelation"),
        ("revelasion", "Revelation"), ("revelashuns", "Revelation"),
    ];

    for (k, v) in mappings {
        m.insert(k.to_string(), v);
    }
    m
});

static PHONETIC_TERMS_RE: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    let terms = [
        ("chaptah", "chapter"),
        ("chapta", "chapter"),
        ("capter", "chapter"),
        ("chaptar", "chapter"),
        ("chapt", "chapter"),
        ("verses", "verse"),
        ("versus", "verse"),
        ("veses", "verse"),
        ("versez", "verse"),
        ("ves", "verse"),
        ("vas", "verse"),
        ("vers", "verse"),
        ("vase", "verse"),
    ];
    terms
        .into_iter()
        .map(|(from, to)| (Regex::new(&format!(r"\b{}\b", from)).unwrap(), to))
        .collect()
});

static BASE_NUMBERS_RE: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    let base = [
        ("nineteen", "19"), ("eighteen", "18"), ("seventeen", "17"), ("sixteen", "16"),
        ("fifteen", "15"), ("fourteen", "14"), ("thirteen", "13"), ("twelve", "12"),
        ("eleven", "11"), ("ten", "10"), ("nine", "9"), ("eight", "8"), ("seven", "7"),
        ("six", "6"), ("five", "5"), ("four", "4"), ("three", "3"), ("two", "2"),
        ("one", "1"),
    ];
    base
        .into_iter()
        .map(|(from, to)| (Regex::new(&format!(r"\b{}\b", from)).unwrap(), to))
        .collect()
});

static CONTEXTUAL_NUMERALS_RE: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    let contextual = [
        (r"(?i)\b(chapter|verse|verses)\s+tree\b", "$1 3"),
        (r"(?i)\b(chapter|verse|verses)\s+eat\b", "$1 8"),
        (r"(?i)\b(chapter|verse|verses)\s+ate\b", "$1 8"),
        (r"(?i)\b(chapter|verse|verses)\s+won\b", "$1 1"),
        (r"(?i)\b(chapter|verse|verses)\s+too\b", "$1 2"),
        (r"(?i)\b(chapter|verse|verses)\s+for\b", "$1 4"),
        (r"(?i)\b(chapter|verse|verses)\s+first\b", "$1 1"),
        (r"(?i)\b(chapter|verse|verses)\s+second\b", "$1 2"),
        (r"(?i)\b(chapter|verse|verses)\s+third\b", "$1 3"),
        (r"(?i)\b(chapter|verse|verses)\s+fourth\b", "$1 4"),
        (r"(?i)\b(chapter|verse|verses)\s+fifth\b", "$1 5"),
    ];
    contextual
        .into_iter()
        .map(|(pat, rep)| (Regex::new(pat).unwrap(), rep))
        .collect()
});

/// Cleans raw Whisper audio transcripts by stripping sound tags/brackets,
/// removing West African church noise filler words,
/// standardizing dashes and colons between digits, and converting other punctuation to spaces.
pub fn clean_spoken_transcript(input: &str) -> String {
    // 1. Remove bracketed audio tags e.g. [music], (applause), (laughter), [blank_audio]
    let bracket_re = Regex::new(r"\[.*?\]|\(.*?\)|<.*?>").unwrap();
    let no_brackets = bracket_re.replace_all(input, " ");

    // 2. Standardize em-dash and en-dash to hyphen
    let dash_std = no_brackets.replace(['—', '–'], "-");

    // 3. Normalize semicolons between digits to colons (Whisper sometimes types 3;16)
    let semi_re = Regex::new(r"(\d+)\s*;\s*(\d+)").unwrap();
    let colons_fixed = semi_re.replace_all(&dash_std, "$1:$2");

    // 4. Strip church filler and noise phrases (prevalent in West African/Ghanaian sermons)
    let mut text_lower = colons_fixed.to_lowercase();
    for noise in [
        "turn with me to the book of",
        "turn with me to",
        "let us turn with me to",
        "let us turn to the book of",
        "let us turn to",
        "let's turn to the book of",
        "let's turn to",
        "turn your bibles to the book of",
        "turn your bibles to",
        "turn your bible to the book of",
        "turn your bible to",
        "turn to the book of",
        "turn to",
        "let us open our bibles to the book of",
        "let us open our bibles to",
        "let us open our bible to the book of",
        "let us open our bible to",
        "let us open to the book of",
        "let us open to",
        "open your bibles to the book of",
        "open your bibles to",
        "open your bible to the book of",
        "open your bible to",
        "open our bibles to the book of",
        "open our bibles to",
        "open our bible to",
        "open to the book of",
        "open to",
        "go with me to the book of",
        "go with me to",
        "let us go to the book of",
        "let us go to",
        "let's go to the book of",
        "let's go to",
        "shall we read from the book of",
        "shall we read from",
        "shall we turn to the book of",
        "shall we turn to",
        "look at the book of",
        "look at",
        "to the book of",
        "the book of",
        "the epistle of",
        "the gospel of",
        "we are reading from the book of",
        "we are reading from",
        "reading from the book of",
        "reading from",
        "let us read from the book of",
        "let us read from",
        "let's read from the book of",
        "let's read from",
        "let us read",
        "let's read",
        "as it is written in the book of",
        "as it is written in",
        "the bible says in the book of",
        "the bible says in",
        "scripture says in the book of",
        "scripture says in",
        "it says in the book of",
        "it says in",
        "praise the lord",
        "praise god",
        "hallelujah",
        "amen",
    ] {
        text_lower = text_lower.replace(noise, " ");
    }

    // 5. Preserve colon only when between digits; convert all other punctuation to spaces
    let mut cleaned = String::with_capacity(text_lower.len());
    let chars: Vec<char> = text_lower.chars().collect();
    for i in 0..chars.len() {
        let c = chars[i];
        if c == ':' {
            let prev_is_digit = i > 0 && chars[i - 1].is_ascii_digit();
            let next_is_digit = i + 1 < chars.len() && chars[i + 1].is_ascii_digit();
            if prev_is_digit && next_is_digit {
                cleaned.push(':');
            } else {
                cleaned.push(' ');
            }
        } else if c == '-' {
            let prev_is_digit = i > 0 && chars[i - 1].is_ascii_digit();
            let next_is_digit = i + 1 < chars.len() && chars[i + 1].is_ascii_digit();
            if prev_is_digit && next_is_digit {
                cleaned.push('-');
            } else {
                cleaned.push(' ');
            }
        } else if c.is_alphanumeric() {
            cleaned.push(c);
        } else {
            cleaned.push(' ');
        }
    }

    // 6. Collapse consecutive spaces and lowercase
    let space_re = Regex::new(r"\s+").unwrap();
    space_re.replace_all(&cleaned, " ").trim().to_string()
}

static SHORTHAND_TREE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b([a-z]+)\s+tree\s+(\d+)\b").unwrap()
});

static BOOK_TREE_STANDALONE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b([a-z]+)\s+tree\b").unwrap()
});

static BOOK_EAT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b([a-z]+)\s+(?:eat|ate)\s+(\d+)\b").unwrap()
});

static BOOK_WON_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b([a-z]+)\s+won\s+(\d+)\b").unwrap()
});

static BOOK_TOO_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b([a-z]+)\s+(?:too|to)\s+(\d+)\b").unwrap()
});

static BOOK_FOR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b([a-z]+)\s+for\s+(\d+)\b").unwrap()
});

const BOOK_PATTERN: &str = r"(?:(?:the\s+)?(?:book|epistle|gospel)\s+of\s+)?(?:saint\s+|st\s+|st\.\s+)?(?:1\s+|2\s+|3\s+)?[a-z]+(?:\s+of\s+[a-z]+)?";

// Explicit pattern: "Book chapter X verse Y", "Book X verse Y", "Book chapter X from verse Y", etc.
static RE_EXPLICIT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"(?i)\b({})\s+(?:chapter\s+(\d+)|(\d+))\s*(?:from|beginning\s+at|starting\s+at|verse|verses|:|\s)+\s*(?:verse\s+|verses\s+)?(\d+)(?:\s*(?:to|through|-|–)\s*(\d+))?",
        BOOK_PATTERN
    )).unwrap()
});

pub const SINGLE_CHAPTER_BOOKS: &[&str] = &["Obadiah", "Philemon", "2 John", "3 John", "Jude"];

// Pattern for single-chapter books without spoken chapter number: "Book verse Y (to Z)"
static RE_SINGLE_CHAPTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"(?i)\b({})\s+(?:verse|verses)\s+(\d+)(?:\s*(?:to|through|-|–)\s*(\d+))?\b",
        BOOK_PATTERN
    )).unwrap()
});

// Shorthand pattern: "Book X:Y" or "Book X Y"
static RE_SHORT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"(?i)\b({})\s+(\d+)\s*[:\s]\s*(\d+)(?:\s*(?:to|through|-|–)\s*(\d+))?",
        BOOK_PATTERN
    )).unwrap()
});

// Inverted pattern: "verse 16 of chapter 3 of John" or "chapter 3 verse 16 of John"
static RE_INVERTED_VERSE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"(?i)\b(?:verse|verses)\s+(\d+)(?:\s*(?:to|through|-)\s*(\d+))?\s+of\s+chapter\s+(\d+)\s+(?:of|in|from)\s+({})\b",
        BOOK_PATTERN
    )).unwrap()
});

static RE_INVERTED_CHAPTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"(?i)\bchapter\s+(\d+)\s+(?:verse|verses)\s+(\d+)(?:\s*(?:to|through|-)\s*(\d+))?\s+(?:of|in|from)\s+({})\b",
        BOOK_PATTERN
    )).unwrap()
});

static RE_CHAPTER_EXPLICIT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"(?i)\b({})\s+chapter\s+(\d+)\b",
        BOOK_PATTERN
    )).unwrap()
});

static RE_PSALM_CHAPTER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(psalms?|proverbs?|sam|salms?)\s+(\d+)\b").unwrap()
});

static RE_CONTINUATION_VERSE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:now\s+in\s+|and\s+in\s+|in\s+|at\s+|and\s+)?(?:verse|verses)\s+(\d+)(?:\s*(?:to|through|-)\s*(\d+))?\b").unwrap()
});

// Converts spoken numeral phrases (e.g., "twenty three" -> "23", "first" -> "1")
// with deep phonetic accent normalization (e.g. Ghanaian/West African "tree" -> "3", "eat" -> "8")
pub fn normalize_spoken_numbers(input: &str) -> String {
    let mut text = clean_spoken_transcript(input);
    
    // 1. Spoken Ordinal Psalms & Proverbs (e.g. "twenty third psalm" -> "psalm 23", "91st psalm" -> "psalm 91")
    let ordinal_psalms = [
        ("twenty third psalm", "psalm 23"), ("23rd psalm", "psalm 23"),
        ("ninety first psalm", "psalm 91"), ("91st psalm", "psalm 91"),
        ("one hundred and nineteenth psalm", "psalm 119"), ("one hundred nineteenth psalm", "psalm 119"),
        ("one hundred and third psalm", "psalm 103"), ("one hundred third psalm", "psalm 103"),
        ("first psalm", "psalm 1"), ("second psalm", "psalm 2"), ("third psalm", "psalm 3"),
    ];
    for (from, to) in ordinal_psalms {
        text = text.replace(from, to);
    }

    // 2. Phonetic word corrections for liturgical terminology
    for (re, to) in PHONETIC_TERMS_RE.iter() {
        text = re.replace_all(&text, *to).to_string();
    }

    // 3. Ordinal prefixes for numbered books
    let book_ordinals = [
        ("first ", "1 "),
        ("1st ", "1 "),
        ("fess ", "1 "),
        ("fes ", "1 "),
        ("fist ", "1 "),
        ("fest ", "1 "),
        ("fas ", "1 "),
        ("fast ", "1 "),
        ("fost ", "1 "),
        ("second ", "2 "),
        ("2nd ", "2 "),
        ("seken ", "2 "),
        ("seknd ", "2 "),
        ("sekken ", "2 "),
        ("sekond ", "2 "),
        ("sekon ", "2 "),
        ("secon ", "2 "),
        ("third ", "3 "),
        ("3rd ", "3 "),
        ("tud ", "3 "),
        ("toid ", "3 "),
        ("tird ", "3 "),
        ("ted ", "3 "),
        ("turd ", "3 "),
        ("trid ", "3 "),
    ];
    for (from, to) in book_ordinals {
        text = text.replace(from, to);
    }

    // 4. Phonetic compound numbers ("twenty tree" -> "23", "twenty eat" -> "28", etc.)
    let accent_compounds = [
        ("twenty tree", "23"), ("thirty tree", "33"), ("forty tree", "43"),
        ("fifty tree", "53"), ("sixty tree", "63"), ("seventy tree", "73"),
        ("eighty tree", "83"), ("ninety tree", "93"),
        ("twenty eat", "28"), ("thirty eat", "38"), ("forty eat", "48"),
        ("fifty eat", "58"), ("sixty eat", "68"), ("seventy eat", "78"),
        ("eighty eat", "88"), ("ninety eat", "98"),
        ("twenty ate", "28"), ("thirty ate", "38"), ("forty ate", "48"),
        ("fifty ate", "58"), ("sixty ate", "68"), ("seventy ate", "78"),
        ("eighty ate", "88"), ("ninety ate", "98"),
        ("terteen", "13"), ("tarteen", "13"),
        ("forteen", "14"),
        ("fiveteen", "15"),
        ("sicksteen", "16"),
    ];
    for (from, to) in accent_compounds {
        text = text.replace(from, to);
    }

    // 5. Spoken compound hundreds
    let compound_hundreds = [
        ("one hundred and nineteen", "119"), ("one hundred nineteen", "119"),
        ("one hundred and twenty", "120"), ("one hundred twenty", "120"),
        ("one hundred and fifty", "150"), ("one hundred fifty", "150"),
        ("one hundred and five", "105"), ("one hundred five", "105"),
        ("one hundred and ", "100 "), ("one hundred ", "100 "),
        ("one hundred", "100"), ("hundred", "100"),
    ];
    for (from, to) in compound_hundreds {
        text = text.replace(from, to);
    }

    // 6. Standard compound numbers 20-99
    let compound_numbers = [
        ("twenty one", "21"), ("twenty two", "22"), ("twenty three", "23"), ("twenty four", "24"),
        ("twenty five", "25"), ("twenty six", "26"), ("twenty seven", "27"), ("twenty eight", "28"),
        ("twenty nine", "29"), ("twenty", "20"),
        ("thirty one", "31"), ("thirty two", "32"), ("thirty three", "33"), ("thirty four", "34"),
        ("thirty five", "35"), ("thirty six", "36"), ("thirty seven", "37"), ("thirty eight", "38"),
        ("thirty nine", "39"), ("thirty", "30"),
        ("forty one", "41"), ("forty two", "42"), ("forty three", "43"), ("forty four", "44"),
        ("forty five", "45"), ("forty six", "46"), ("forty seven", "47"), ("forty eight", "48"),
        ("forty nine", "49"), ("forty", "40"),
        ("fifty one", "51"), ("fifty two", "52"), ("fifty three", "53"), ("fifty four", "54"),
        ("fifty five", "55"), ("fifty six", "56"), ("fifty seven", "57"), ("fifty eight", "58"),
        ("fifty nine", "59"), ("fifty", "50"),
        ("sixty one", "61"), ("sixty two", "62"), ("sixty three", "63"), ("sixty four", "64"),
        ("sixty five", "65"), ("sixty six", "66"), ("sixty seven", "67"), ("sixty eight", "68"),
        ("sixty nine", "69"), ("sixty", "60"),
        ("seventy one", "71"), ("seventy two", "72"), ("seventy three", "73"), ("seventy four", "74"),
        ("seventy five", "75"), ("seventy six", "76"), ("seventy seven", "77"), ("seventy eight", "78"),
        ("seventy nine", "79"), ("seventy", "70"),
        ("eighty one", "81"), ("eighty two", "82"), ("eighty three", "83"), ("eighty four", "84"),
        ("eighty five", "85"), ("eighty six", "86"), ("eighty seven", "87"), ("eighty eight", "88"),
        ("eighty nine", "89"), ("eighty", "80"),
        ("ninety one", "91"), ("ninety two", "92"), ("ninety three", "93"), ("ninety four", "94"),
        ("ninety five", "95"), ("ninety six", "96"), ("ninety seven", "97"), ("ninety eight", "98"),
        ("ninety nine", "99"), ("ninety", "90"),
    ];
    for (from, to) in compound_numbers {
        text = text.replace(from, to);
    }

    // 7. Single / teen numbers
    for (re, to) in BASE_NUMBERS_RE.iter() {
        text = re.replace_all(&text, *to).to_string();
    }

    // 8. Contextual phonetic numerals
    for (re, rep) in CONTEXTUAL_NUMERALS_RE.iter() {
        text = re.replace_all(&text, *rep).to_string();
    }

    // 9. Book-adjacent Ghanaian phonemes: e.g. "John tree 16" -> "John 3 16", "Romans eat 28" -> "Romans 8 28"
    text = SHORTHAND_TREE_RE.replace_all(&text, "$1 3 $2").to_string();
    text = BOOK_EAT_RE.replace_all(&text, "$1 8 $2").to_string();
    text = BOOK_WON_RE.replace_all(&text, "$1 1 $2").to_string();

    // Only convert "too" / "to" or "for" after an actual canonical Bible book name to avoid mangling prepositions
    if let Some(caps) = BOOK_TOO_RE.captures(&text) {
        let candidate_book = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        if resolve_canonical_book(candidate_book, false).is_some() {
            text = BOOK_TOO_RE.replace_all(&text, "$1 2 $2").to_string();
        }
    }
    if let Some(caps) = BOOK_FOR_RE.captures(&text) {
        let candidate_book = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        if resolve_canonical_book(candidate_book, false).is_some() {
            text = BOOK_FOR_RE.replace_all(&text, "$1 4 $2").to_string();
        }
    }

    // Check standalone "tree" after known books e.g. "John tree" -> "John 3"
    if let Some(caps) = BOOK_TREE_STANDALONE_RE.captures(&text) {
        let candidate_book = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        if resolve_canonical_book(candidate_book, false).is_some() {
            text = BOOK_TREE_STANDALONE_RE.replace_all(&text, "$1 3").to_string();
        }
    }

    text
}

/// Resolves spoken or misspelled book text to its canonical title using strsim (Jaro-Winkler + Levenshtein).
/// If `allow_fuzzy` is false, only exact canonical titles and verified aliases in BOOK_ALIASES are matched.
pub fn resolve_canonical_book(raw: &str, allow_fuzzy: bool) -> Option<&'static str> {
    let mut s = raw.trim().to_lowercase();
    let mut has_liturgical_prefix = false;
    for prefix in [
        "the book of ",
        "book of ",
        "the epistle of ",
        "epistle of ",
        "the gospel of ",
        "gospel of ",
        "saint ",
        "st. ",
        "st ",
    ] {
        if s.starts_with(prefix) {
            has_liturgical_prefix = true;
            s = s[prefix.len()..].trim().to_string();
        }
    }

    // Prevent stop-words from ever falsely matching Bible books
    if STOPWORDS_BLACKLIST.contains(s.as_str()) {
        return None;
    }

    // 1. Direct dictionary match against canonical names and aliases
    if let Some(&canonical) = BOOK_ALIASES.get(s.as_str()) {
        return Some(canonical);
    }

    // 2. Trailing 's' strip (e.g. "Romans" vs "Roman", "Hebrews" vs "Hebrew")
    if s.ends_with('s') {
        let without_s = &s[..s.len() - 1];
        if let Some(&canonical) = BOOK_ALIASES.get(without_s) {
            return Some(canonical);
        }
    }

    // 3. Prefix stripping for numbered books (e.g. "first peter", "seken corinthians", "1 peter")
    let first_prefixes = ["1 ", "1st ", "first ", "fess ", "fes ", "fist ", "fest ", "fas ", "fast ", "fost "];
    let second_prefixes = ["2 ", "2nd ", "second ", "seken ", "seknd ", "sekken ", "sekond ", "sekon ", "secon "];
    let third_prefixes = ["3 ", "3rd ", "third ", "tud ", "toid ", "tird ", "ted ", "turd ", "trid "];

    let mut prefix_num = None;
    let mut remainder = s.as_str();

    for p in first_prefixes {
        if s.starts_with(p) {
            prefix_num = Some(1);
            remainder = s[p.len()..].trim();
            break;
        }
    }
    if prefix_num.is_none() {
        for p in second_prefixes {
            if s.starts_with(p) {
                prefix_num = Some(2);
                remainder = s[p.len()..].trim();
                break;
            }
        }
    }
    if prefix_num.is_none() {
        for p in third_prefixes {
            if s.starts_with(p) {
                prefix_num = Some(3);
                remainder = s[p.len()..].trim();
                break;
            }
        }
    }

    if let Some(num) = prefix_num {
        if let Some(&canonical) = BOOK_ALIASES.get(remainder) {
            let target = format!("{} {}", num, canonical.trim_start_matches(|c: char| c.is_ascii_digit() || c.is_whitespace()));
            for &b in CANONICAL_BOOKS.iter() {
                if b.eq_ignore_ascii_case(&target) {
                    return Some(b);
                }
            }
        }
    }

    // 4. Fuzzy Levenshtein + Jaro-Winkler match with length-based thresholds
    if allow_fuzzy || has_liturgical_prefix {
        let s_len = s.len();
        if s_len >= 3 {
            let mut best_match: Option<&'static str> = None;
            let mut highest_score = 0.0f64;

            for &canonical in CANONICAL_BOOKS.iter() {
                let can_lower = canonical.to_lowercase();
                let jw = jaro_winkler(&s, &can_lower);
                let dist = strsim_levenshtein(&s, &can_lower);

                let is_match = if s_len <= 4 {
                    dist <= 1 && jw >= 0.88
                } else if s_len <= 7 {
                    dist <= 2 || jw >= 0.85
                } else {
                    dist <= 3 || jw >= 0.82
                };

                if is_match && jw > highest_score {
                    highest_score = jw;
                    best_match = Some(canonical);
                }
            }

            if let Some(matched) = best_match {
                return Some(matched);
            }
        }
    }

    None
}

/// Matches continuation phrases like "now in verse thirty" or "verse 30"
/// using the active book and chapter context
pub fn parse_continuation_verse(transcript: &str, active_book: &str, active_chapter: u32) -> Option<SpokenCitation> {
    let normalized = normalize_spoken_numbers(transcript);
    if let Some(caps) = RE_CONTINUATION_VERSE.captures(&normalized) {
        let verse_start: u32 = caps.get(1)?.as_str().parse().ok()?;
        let verse_end: Option<u32> = caps.get(2).and_then(|m| m.as_str().parse().ok());
        if verse_start >= 1 && verse_start <= 176 {
            return Some(SpokenCitation {
                book: active_book.to_string(),
                chapter: active_chapter,
                verse_start,
                verse_end,
                confidence: 0.90,
                raw_match: caps.get(0)?.as_str().to_string(),
            });
        }
    }
    None
}

/// Matches Bible references in transcribed spoken speech with deep accent resilience
pub fn parse_spoken_scripture(transcript: &str) -> Option<SpokenCitation> {
    let normalized = normalize_spoken_numbers(transcript);

    // Pattern 1: Explicit "Book chapter X verse Y (to Z)" or "Book X verse Y" or "Book X from verse Y"
    for caps in RE_EXPLICIT.captures_iter(&normalized) {
        if let Some(m) = caps.get(1) {
            let raw_book = m.as_str();
            if let Some(canonical_book) = resolve_canonical_book(raw_book, true) {
                let chapter: Option<u32> = if let Some(c) = caps.get(2) {
                    c.as_str().parse().ok()
                } else if let Some(c) = caps.get(3) {
                    c.as_str().parse().ok()
                } else {
                    None
                };
                let verse_start: Option<u32> = caps.get(4).and_then(|v| v.as_str().parse().ok());
                if let (Some(ch), Some(vs)) = (chapter, verse_start) {
                    let verse_end: Option<u32> = caps.get(5).and_then(|m| m.as_str().parse().ok());
                    return Some(SpokenCitation {
                        book: canonical_book.to_string(),
                        chapter: ch,
                        verse_start: vs,
                        verse_end,
                        confidence: 0.95,
                        raw_match: caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default(),
                    });
                }
            }
        }
    }

    // Pattern 1b: Single-chapter books with "verse X" (e.g. "Jude verse 24", "Philemon verse 6", "3 John verse 2")
    for caps in RE_SINGLE_CHAPTER.captures_iter(&normalized) {
        if let Some(m) = caps.get(1) {
            let raw_book = m.as_str();
            if let Some(canonical_book) = resolve_canonical_book(raw_book, true) {
                if SINGLE_CHAPTER_BOOKS.contains(&canonical_book) {
                    if let Some(verse_start) = caps.get(2).and_then(|v| v.as_str().parse::<u32>().ok()) {
                        let verse_end = caps.get(3).and_then(|v| v.as_str().parse::<u32>().ok());
                        return Some(SpokenCitation {
                            book: canonical_book.to_string(),
                            chapter: 1,
                            verse_start,
                            verse_end,
                            confidence: 0.95,
                            raw_match: caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default(),
                        });
                    }
                }
            }
        }
    }

    // Pattern 2: Inverted phrases like "verse 16 of chapter 3 of John"
    for caps in RE_INVERTED_VERSE.captures_iter(&normalized) {
        let verse_start = caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok());
        let verse_end = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok());
        let chapter = caps.get(3).and_then(|m| m.as_str().parse::<u32>().ok());
        if let (Some(vs), Some(ch), Some(raw_book)) = (verse_start, chapter, caps.get(4).map(|m| m.as_str())) {
            if let Some(canonical_book) = resolve_canonical_book(raw_book, true) {
                return Some(SpokenCitation {
                    book: canonical_book.to_string(),
                    chapter: ch,
                    verse_start: vs,
                    verse_end,
                    confidence: 0.95,
                    raw_match: caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default(),
                });
            }
        }
    }

    // Pattern 3: Inverted phrases like "chapter 3 verse 16 of John"
    for caps in RE_INVERTED_CHAPTER.captures_iter(&normalized) {
        let chapter = caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok());
        let verse_start = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok());
        let verse_end = caps.get(3).and_then(|m| m.as_str().parse::<u32>().ok());
        if let (Some(ch), Some(vs), Some(raw_book)) = (chapter, verse_start, caps.get(4).map(|m| m.as_str())) {
            if let Some(canonical_book) = resolve_canonical_book(raw_book, true) {
                return Some(SpokenCitation {
                    book: canonical_book.to_string(),
                    chapter: ch,
                    verse_start: vs,
                    verse_end,
                    confidence: 0.95,
                    raw_match: caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default(),
                });
            }
        }
    }

    // Pattern 4: Shorthand "Book X:Y" or "Book X Y" (e.g. "John 3:16", "Romans 8 28")
    for caps in RE_SHORT.captures_iter(&normalized) {
        if let Some(raw_book) = caps.get(1).map(|m| m.as_str()) {
            if let Some(canonical_book) = resolve_canonical_book(raw_book, false) {
                let chapter = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok());
                let verse_start = caps.get(3).and_then(|m| m.as_str().parse::<u32>().ok());
                let verse_end = caps.get(4).and_then(|m| m.as_str().parse::<u32>().ok());
                if let (Some(ch), Some(vs)) = (chapter, verse_start) {
                    return Some(SpokenCitation {
                        book: canonical_book.to_string(),
                        chapter: ch,
                        verse_start: vs,
                        verse_end,
                        confidence: 0.85,
                        raw_match: caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default(),
                    });
                }
            }
        }
    }

    // Pattern 5a: Explicit Chapter Reference (e.g. "Romans chapter 8", "John chapter 3")
    for caps in RE_CHAPTER_EXPLICIT.captures_iter(&normalized) {
        if let Some(raw_book) = caps.get(1).map(|m| m.as_str()) {
            if let Some(canonical_book) = resolve_canonical_book(raw_book, true) {
                if let Some(chapter) = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok()) {
                    return Some(SpokenCitation {
                        book: canonical_book.to_string(),
                        chapter,
                        verse_start: 1,
                        verse_end: None,
                        confidence: 0.80,
                        raw_match: caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default(),
                    });
                }
            }
        }
    }

    // Pattern 5b: Psalms / Proverbs bare chapter reference (e.g. "Psalm 23", "Sam 23", "Proverbs 3")
    for caps in RE_PSALM_CHAPTER.captures_iter(&normalized) {
        if let Some(raw_book) = caps.get(1).map(|m| m.as_str()) {
            if let Some(canonical_book) = resolve_canonical_book(raw_book, false) {
                if let Some(chapter) = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok()) {
                    return Some(SpokenCitation {
                        book: canonical_book.to_string(),
                        chapter,
                        verse_start: 1,
                        verse_end: None,
                        confidence: 0.85,
                        raw_match: caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default(),
                    });
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spoken_numbers_normalization() {
        assert_eq!(normalize_spoken_numbers("twenty three"), "23");
        assert_eq!(normalize_spoken_numbers("chapter three verse sixteen"), "chapter 3 verse 16");
        assert_eq!(normalize_spoken_numbers("first corinthians"), "1 corinthians");
        assert_eq!(normalize_spoken_numbers("second kings"), "2 kings");
    }

    #[test]
    fn test_john_3_16() {
        let text = "Please turn your Bibles to John chapter three verse sixteen for God so loved";
        let parsed = parse_spoken_scripture(text).expect("Should detect John 3:16");
        assert_eq!(parsed.book, "John");
        assert_eq!(parsed.chapter, 3);
        assert_eq!(parsed.verse_start, 16);
        assert_eq!(parsed.verse_end, None);
    }

    #[test]
    fn test_romans_range() {
        let text = "In the book of Romans chapter eight verse twenty eight to thirty we see";
        let parsed = parse_spoken_scripture(text).expect("Should detect Romans 8:28-30");
        assert_eq!(parsed.book, "Romans");
        assert_eq!(parsed.chapter, 8);
        assert_eq!(parsed.verse_start, 28);
        assert_eq!(parsed.verse_end, Some(30));
    }

    #[test]
    fn test_ordinal_book_first_corinthians() {
        let text = "turn with me to first corinthians chapter thirteen verse four";
        let parsed = parse_spoken_scripture(text).expect("Should detect 1 Corinthians 13:4");
        assert_eq!(parsed.book, "1 Corinthians");
        assert_eq!(parsed.chapter, 13);
        assert_eq!(parsed.verse_start, 4);
    }

    #[test]
    fn test_shorthand_citation() {
        let text = "as written in Psalm 23:1";
        let parsed = parse_spoken_scripture(text).expect("Should detect Psalm 23:1");
        assert_eq!(parsed.book, "Psalms");
        assert_eq!(parsed.chapter, 23);
        assert_eq!(parsed.verse_start, 1);
    }

    #[test]
    fn test_shorthand_spoken_without_colon() {
        let text = "turn to John 3 16 right now";
        let parsed = parse_spoken_scripture(text).expect("Should detect John 3:16 shorthand");
        assert_eq!(parsed.book, "John");
        assert_eq!(parsed.chapter, 3);
        assert_eq!(parsed.verse_start, 16);
    }

    #[test]
    fn test_reject_common_words_false_positives() {
        // Words like "join", "make", "take", "truth" must NOT match in shorthand
        assert!(parse_spoken_scripture("join 3 16").is_none());
        assert!(parse_spoken_scripture("make 1 2").is_none());
        assert!(parse_spoken_scripture("take 2 3").is_none());
        assert!(parse_spoken_scripture("the truth 2 3").is_none());
    }

    #[test]
    fn test_bare_chapter_behavior() {
        // "Romans chapter 8" should match chapter 8
        let parsed = parse_spoken_scripture("Romans chapter 8").expect("Should detect Romans chapter 8");
        assert_eq!(parsed.book, "Romans");
        assert_eq!(parsed.chapter, 8);
        assert_eq!(parsed.verse_start, 1);

        // Bare "Romans 8" without "chapter" should NOT eagerly match chapter,
        // leaving it in context for the verse number
        assert!(parse_spoken_scripture("Romans 8").is_none());

        // Psalms bare number "Psalm 23" should match Psalms 23:1
        let psalm = parse_spoken_scripture("Psalm 23").expect("Should detect Psalm 23");
        assert_eq!(psalm.book, "Psalms");
        assert_eq!(psalm.chapter, 23);
        assert_eq!(psalm.verse_start, 1);
    }

    #[test]
    fn test_ghanaian_accent_tree_sixteen() {
        let text = "turn with me to John chapter tree verse sixteen";
        let parsed = parse_spoken_scripture(text).expect("Should detect John 3:16 with 'tree' phoneme");
        assert_eq!(parsed.book, "John");
        assert_eq!(parsed.chapter, 3);
        assert_eq!(parsed.verse_start, 16);
    }

    #[test]
    fn test_ghanaian_accent_romans_eat() {
        let text = "in the book of Romance chapter eat verse twenty eat";
        let parsed = parse_spoken_scripture(text).expect("Should detect Romans 8:28 with 'eat' phoneme and Romance fuzzy match");
        assert_eq!(parsed.book, "Romans");
        assert_eq!(parsed.chapter, 8);
        assert_eq!(parsed.verse_start, 28);
    }

    #[test]
    fn test_ghanaian_accent_sam_twenty_three() {
        let text = "let us read from Sam twenty tree";
        let parsed = parse_spoken_scripture(text).expect("Should detect Psalm 23 with 'Sam' and 'tree'");
        assert_eq!(parsed.book, "Psalms");
        assert_eq!(parsed.chapter, 23);
        assert_eq!(parsed.verse_start, 1);
    }

    #[test]
    fn test_ghanaian_accent_colinthians() {
        let text = "open to first colinthians chapter thirteen verse four";
        let parsed = parse_spoken_scripture(text).expect("Should detect 1 Corinthians 13:4 with 'colinthians'");
        assert_eq!(parsed.book, "1 Corinthians");
        assert_eq!(parsed.chapter, 13);
        assert_eq!(parsed.verse_start, 4);
    }

    #[test]
    fn test_ghanaian_accent_hebru() {
        let text = "in hebru chapter eleven verse won";
        let parsed = parse_spoken_scripture(text).expect("Should detect Hebrews 11:1 with 'hebru' and 'won'");
        assert_eq!(parsed.book, "Hebrews");
        assert_eq!(parsed.chapter, 11);
        assert_eq!(parsed.verse_start, 1);
    }

    #[test]
    fn test_spoken_words_numbers() {
        // "john three sixteen"
        let parsed1 = parse_spoken_scripture("john three sixteen").expect("Should detect John 3:16 from spoken words");
        assert_eq!(parsed1.book, "John");
        assert_eq!(parsed1.chapter, 3);
        assert_eq!(parsed1.verse_start, 16);

        // "romans eight twenty eight"
        let parsed2 = parse_spoken_scripture("romans eight twenty eight").expect("Should detect Romans 8:28");
        assert_eq!(parsed2.book, "Romans");
        assert_eq!(parsed2.chapter, 8);
        assert_eq!(parsed2.verse_start, 28);

        // "matthew six thirty three"
        let parsed3 = parse_spoken_scripture("matthew six thirty three").expect("Should detect Matthew 6:33");
        assert_eq!(parsed3.book, "Matthew");
        assert_eq!(parsed3.chapter, 6);
        assert_eq!(parsed3.verse_start, 33);

        // "philippians four thirteen"
        let parsed4 = parse_spoken_scripture("philippians four thirteen").expect("Should detect Philippians 4:13");
        assert_eq!(parsed4.book, "Philippians");
        assert_eq!(parsed4.chapter, 4);
        assert_eq!(parsed4.verse_start, 13);
    }

    #[test]
    fn test_punctuation_whisper_noise() {
        // "John, 3:16." with comma and trailing period
        let parsed1 = parse_spoken_scripture("John, 3:16.").expect("Should detect John 3:16 with punctuation");
        assert_eq!(parsed1.book, "John");
        assert_eq!(parsed1.chapter, 3);
        assert_eq!(parsed1.verse_start, 16);

        // "John, three sixteen."
        let parsed2 = parse_spoken_scripture("John, three sixteen.").expect("Should detect John 3:16 with comma & period");
        assert_eq!(parsed2.book, "John");
        assert_eq!(parsed2.chapter, 3);
        assert_eq!(parsed2.verse_start, 16);

        // "John 3, 16"
        let parsed3 = parse_spoken_scripture("John 3, 16").expect("Should detect John 3:16 with comma separator");
        assert_eq!(parsed3.book, "John");
        assert_eq!(parsed3.chapter, 3);
        assert_eq!(parsed3.verse_start, 16);

        // "[Music] turn to John 3:16 (applause)"
        let parsed4 = parse_spoken_scripture("[Music] please turn to John 3:16 (applause)").expect("Should ignore audio tags");
        assert_eq!(parsed4.book, "John");
        assert_eq!(parsed4.chapter, 3);
        assert_eq!(parsed4.verse_start, 16);
    }

    #[test]
    fn test_ordinal_psalms() {
        let parsed1 = parse_spoken_scripture("let us read the twenty third psalm").expect("Should detect Psalm 23 from 'twenty third psalm'");
        assert_eq!(parsed1.book, "Psalms");
        assert_eq!(parsed1.chapter, 23);
        assert_eq!(parsed1.verse_start, 1);

        let parsed2 = parse_spoken_scripture("open to the 91st psalm").expect("Should detect Psalm 91");
        assert_eq!(parsed2.book, "Psalms");
        assert_eq!(parsed2.chapter, 91);
        assert_eq!(parsed2.verse_start, 1);
    }

    #[test]
    fn test_inverted_citations() {
        let parsed1 = parse_spoken_scripture("verse sixteen of chapter three of John").expect("Should detect inverted John 3:16");
        assert_eq!(parsed1.book, "John");
        assert_eq!(parsed1.chapter, 3);
        assert_eq!(parsed1.verse_start, 16);

        let parsed2 = parse_spoken_scripture("chapter eight verse twenty eight in the book of Romans").expect("Should detect Romans 8:28");
        assert_eq!(parsed2.book, "Romans");
        assert_eq!(parsed2.chapter, 8);
        assert_eq!(parsed2.verse_start, 28);
    }

    #[test]
    fn test_conversational_connectors() {
        let parsed1 = parse_spoken_scripture("please turn your bibles to the book of John three sixteen for today").expect("Should detect in conversational sentence");
        assert_eq!(parsed1.book, "John");
        assert_eq!(parsed1.chapter, 3);
        assert_eq!(parsed1.verse_start, 16);

        let parsed2 = parse_spoken_scripture("Romans 8 from verse 28").expect("Should detect with 'from verse'");
        assert_eq!(parsed2.book, "Romans");
        assert_eq!(parsed2.chapter, 8);
        assert_eq!(parsed2.verse_start, 28);
    }

    #[test]
    fn test_continuation_verse() {
        let parsed1 = parse_continuation_verse("now in verse thirty", "Romans", 8).expect("Should detect verse 30 continuation");
        assert_eq!(parsed1.book, "Romans");
        assert_eq!(parsed1.chapter, 8);
        assert_eq!(parsed1.verse_start, 30);

        let parsed2 = parse_continuation_verse("and verse 29", "Romans", 8).expect("Should detect verse 29");
        assert_eq!(parsed2.book, "Romans");
        assert_eq!(parsed2.chapter, 8);
        assert_eq!(parsed2.verse_start, 29);
    }

    #[test]
    fn test_ghanaian_tree_shorthand() {
        let parsed1 = parse_spoken_scripture("turn to john tree sixteen").expect("Should detect John 3:16 with 'tree'");
        assert_eq!(parsed1.book, "John");
        assert_eq!(parsed1.chapter, 3);
        assert_eq!(parsed1.verse_start, 16);

        let parsed2 = parse_spoken_scripture("open to john chapter tree").expect("Should detect John 3 with 'chapter tree'");
        assert_eq!(parsed2.book, "John");
        assert_eq!(parsed2.chapter, 3);
        assert_eq!(parsed2.verse_start, 1);
    }

    #[test]
    fn test_spoken_ghanaian_first_samuel() {
        let text = "please turn with me to first samuel chapter seventeen verse forty five";
        let parsed = parse_spoken_scripture(text).expect("Should detect 1 Samuel 17:45");
        assert_eq!(parsed.book, "1 Samuel");
        assert_eq!(parsed.chapter, 17);
        assert_eq!(parsed.verse_start, 45);
        assert_eq!(parsed.verse_end, None);
    }

    #[test]
    fn test_spoken_ghanaian_seken_corinthians() {
        let text = "open your bibles to seken corinthians chapter tree verse eat";
        let parsed = parse_spoken_scripture(text).expect("Should detect 2 Corinthians 3:8 with 'seken', 'tree', 'eat'");
        assert_eq!(parsed.book, "2 Corinthians");
        assert_eq!(parsed.chapter, 3);
        assert_eq!(parsed.verse_start, 8);
        assert_eq!(parsed.verse_end, None);
    }

    #[test]
    fn test_spoken_first_thessalonians_range() {
        let text = "let us read from first thessalonians chapter five verse sixteen to eighteen";
        let parsed = parse_spoken_scripture(text).expect("Should detect 1 Thessalonians 5:16-18");
        assert_eq!(parsed.book, "1 Thessalonians");
        assert_eq!(parsed.chapter, 5);
        assert_eq!(parsed.verse_start, 16);
        assert_eq!(parsed.verse_end, Some(18));
    }

    #[test]
    fn test_spoken_efisians_accent() {
        let text = "reading from the book of efisians chapter two verse eight";
        let parsed = parse_spoken_scripture(text).expect("Should detect Ephesians 2:8 with 'efisians'");
        assert_eq!(parsed.book, "Ephesians");
        assert_eq!(parsed.chapter, 2);
        assert_eq!(parsed.verse_start, 8);
    }

    #[test]
    fn test_spoken_fess_samuel_shorthand() {
        let text = "fess samuel 17 45";
        let parsed = parse_spoken_scripture(text).expect("Should detect 1 Samuel 17:45 from shorthand 'fess samuel 17 45'");
        assert_eq!(parsed.book, "1 Samuel");
        assert_eq!(parsed.chapter, 17);
        assert_eq!(parsed.verse_start, 45);
    }

    #[test]
    fn test_single_chapter_books() {
        // Jude
        let parsed_jude = parse_spoken_scripture("please turn with me to jude verse twenty four").expect("Should detect Jude 1:24");
        assert_eq!(parsed_jude.book, "Jude");
        assert_eq!(parsed_jude.chapter, 1);
        assert_eq!(parsed_jude.verse_start, 24);

        // Philemon
        let parsed_phm = parse_spoken_scripture("open your bible to philemon verse six").expect("Should detect Philemon 1:6");
        assert_eq!(parsed_phm.book, "Philemon");
        assert_eq!(parsed_phm.chapter, 1);
        assert_eq!(parsed_phm.verse_start, 6);

        // 3 John
        let parsed_3j = parse_spoken_scripture("let us read from third john verse two").expect("Should detect 3 John 1:2");
        assert_eq!(parsed_3j.book, "3 John");
        assert_eq!(parsed_3j.chapter, 1);
        assert_eq!(parsed_3j.verse_start, 2);

        // Obadiah range
        let parsed_ob = parse_spoken_scripture("reading from obadiah verse three to four").expect("Should detect Obadiah 1:3-4");
        assert_eq!(parsed_ob.book, "Obadiah");
        assert_eq!(parsed_ob.chapter, 1);
        assert_eq!(parsed_ob.verse_start, 3);
        assert_eq!(parsed_ob.verse_end, Some(4));
    }
}
