use serde::de::{Error as DeError, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::collections::HashSet;
use std::convert::TryFrom;
use std::fmt;
use std::marker::PhantomData;

/// A utility for deserialization of an arbitrary amount of `T`, expecting at least one item.
///
/// The type is deserialized from a [`String`] with the [`TryFrom`] trait.
///
/// [`TryFrom`]: std::convert::TryFrom
/// [`String`]: std::string::String
#[derive(Debug, Clone)]
pub struct OneOrMore<T>(pub Vec<T>);

impl<T> Serialize for OneOrMore<T>
where
    T: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.0.len() == 1 {
            Serialize::serialize(&(self.0)[0], serializer)
        } else {
            let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

            for item in &self.0 {
                seq.serialize_element(item)?;
            }

            seq.end()
        }
    }
}

impl<'de, T> Deserialize<'de> for OneOrMore<T>
where
    T: TryFrom<String>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for V<T>
        where
            T: TryFrom<String>,
        {
            type Value = Vec<T>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("one string or more")
            }

            fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
                let item = match T::try_from(v.to_string()) {
                    Ok(item) => item,
                    Err(_) => return Err(E::custom("failed to parse from string")),
                };

                Ok(vec![item])
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut v = Vec::with_capacity(seq.size_hint().unwrap_or(0));

                while let Some(elem) = seq.next_element::<String>()? {
                    let item = match T::try_from(elem) {
                        Ok(item) => item,
                        Err(_) => return Err(A::Error::custom("failed to parse from string")),
                    };

                    v.push(item);
                }

                assert!(v.len() >= 1, "expected at least one string");

                Ok(v)
            }
        }

        deserializer.deserialize_any(V(PhantomData)).map(OneOrMore)
    }
}

/// Describes a Github author by their name.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Author(String);

impl Author {
    /// Create a new Author with their name.
    #[inline]
    pub fn new<I>(name: I) -> Self
    where
        I: Into<String>,
    {
        Self(name.into())
    }

    /// Access the author's name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Author {
    /// Format the author to display the second part of reference-style link of a Github mention to them.
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[@{}]", self.name())
    }
}

impl TryFrom<String> for Author {
    type Error = std::convert::Infallible;

    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Ok(Self::new(s))
    }
}

/// Describes a Git commit by its hash.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Commit(String);

impl Commit {
    /// Create a new commit with its hash.
    ///
    /// # Panics
    /// A panic is incurred if:
    /// - the passed hash is shorter than 7 characters.
    #[inline]
    pub fn new<I>(hash: I) -> Self
    where
        I: Into<String>,
    {
        let hash = hash.into();
        assert!(
            hash.len() >= 7,
            "commit hashes must not be shorter than 7 characters"
        );
        Self(hash)
    }

    /// Access the commit hash.
    #[inline]
    pub fn hash(&self) -> &str {
        &self.0
    }
}

/// Describes an error when trying to convert to a [`Commit`] from a [`String`].
///
/// [`Commit`]: struct.Commit.html
/// [`String`]: std::string::String
#[derive(Debug, Clone, PartialEq)]
pub struct CommitConversionError(
    /// The offending string that was passed.
    pub String,
);

impl fmt::Display for CommitConversionError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt("commit hashes must not be shorter than 7 characters", f)
    }
}

impl std::error::Error for CommitConversionError {}

impl TryFrom<String> for Commit {
    type Error = CommitConversionError;

    /// Try convert a [`String`] to a [`Commit`].
    ///
    /// # Errors
    /// An error is returned if:
    /// - the passed [`String`] is shorter than 7 characters
    ///
    /// [`Commit`]: struct.Commit.html
    /// [`String`]: std::string::String
    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.len() < 7 {
            return Err(CommitConversionError(s));
        }

        Ok(Self::new(s))
    }
}

impl fmt::Display for Commit {
    /// Format the commit to display the second part of reference-style link to them.
    /// Only the first seven characters of the hash are outputted, for legibility.
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[c:{}]", &self.hash()[..7])
    }
}

/// Represents a change that was applied to a repository.
///
/// The first field describes the location of the change - category.<br>
/// The second field expresses the name of the change - name.<br>
/// The third field specifies the author(s) of the change that participated - authors.<br>
/// The fourth field tells the commit(s) of the change - commits.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Change(
    pub String,
    pub String,
    pub OneOrMore<Author>,
    pub OneOrMore<Commit>,
);

impl Change {
    /// Create a new Change with a category, a name, a single author, and a single commit.
    pub fn new<A, B, C, D>(category: A, name: B, author: C, commit: D) -> Self
    where
        A: Into<String>,
        B: Into<String>,
        C: Into<String>,
        D: Into<String>,
    {
        Self(
            category.into(),
            name.into(),
            OneOrMore(vec![Author::new(author)]),
            OneOrMore(vec![Commit::new(commit)]),
        )
    }
}

/// Represents a release of the software from the current snapshot of the repository.
#[derive(Serialize, Default, Deserialize, Debug, Clone)]
pub struct Release {
    /// The URL to the Github repository.
    pub repo_url: String,
    /// Changes whose purpose was to add functionality.
    #[serde(default)]
    pub added: Vec<Change>,
    /// Changes whose purpose was to change existing functionality.
    #[serde(default)]
    pub changed: Vec<Change>,
    /// Changes whose purpose was to fix existing functionality.
    #[serde(default)]
    pub fixed: Vec<Change>,
    /// Changes whose purpose was to remove existing functionality.
    #[serde(default)]
    pub removed: Vec<Change>,
}

impl Release {
    fn iter(&self) -> impl Iterator<Item = &Change> + '_ {
        self.added
            .iter()
            .chain(self.changed.iter())
            .chain(self.fixed.iter())
            .chain(self.removed.iter())
    }

    /// Return all unique authors of the whole release.
    pub fn get_authors(&self) -> Vec<Author> {
        self.iter()
            .flat_map(|Change(_, _, OneOrMore(authors), _)| authors.iter().cloned())
            .collect::<HashSet<Author>>()
            .into_iter()
            .collect()
    }

    /// Return all commits of the whole release.
    pub fn get_commits(&self) -> Vec<Commit> {
        self.iter()
            .flat_map(|Change(_, _, _, OneOrMore(commits))| commits.iter().cloned())
            .collect()
    }
}

fn write_separated<T, It>(source: &mut dyn fmt::Write, it: It, sep: &str) -> fmt::Result
where
    It: IntoIterator<Item = T>,
    T: fmt::Display,
{
    let it = it.into_iter();
    let mut first = true;

    for elem in it {
        if !first {
            source.write_str(sep)?;
        }

        write!(source, "{}", elem)?;

        first = false;
    }

    Ok(())
}

fn write_list(source: &mut dyn fmt::Write, header: &str, changes: &[Change]) -> fmt::Result {
    if changes.is_empty() {
        return Ok(());
    }

    writeln!(source, "{}\n", header)?;

    for change in changes {
        let Change(category, name, OneOrMore(authors), OneOrMore(commits)) = change;

        assert!(!category.is_empty(), "categores cannot be empty");

        write!(source, "- [{}] {} (", category, name)?;
        write_separated(source, authors, " ")?;
        write!(source, ") ")?;

        write_separated(source, commits, " ")?;

        writeln!(source)?;
    }

    writeln!(source)?;

    Ok(())
}

/// Generate the output message from a [`Release`] by writing to a source implementing
/// [`std::fmt::Write`]
///
/// [`Release`]: struct.Release.html
/// [`std::fmt::Write`]: std::fmt::Write
pub fn generate_msg(source: &mut dyn fmt::Write, rel: &Release) -> fmt::Result {
    writeln!(source, "Thanks to the following for their contributions:\n")?;

    let mut authors = rel.get_authors();
    // Sort authors by their names alphabetically.
    authors.sort_by(|a, b| a.name().to_lowercase().cmp(&b.name().to_lowercase()));

    let commits = rel.get_commits();

    for author in &authors {
        writeln!(source, "- {}", author)?;
    }

    writeln!(source)?;

    write_list(source, "### Added", &rel.added)?;
    write_list(source, "### Changed", &rel.changed)?;
    write_list(source, "### Fixed", &rel.fixed)?;
    write_list(source, "### Removed", &rel.removed)?;

    for author in authors {
        writeln!(source, "{}: https://github.com/{}", author, author.name())?;
    }

    writeln!(source)?;

    for commit in commits {
        writeln!(
            source,
            "{}: {}/commit/{}",
            commit,
            rel.repo_url,
            commit.hash()
        )?;
    }

    Ok(())
}
