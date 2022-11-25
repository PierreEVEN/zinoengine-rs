use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut, Range};

/// Character to separate path segments
pub const PATH_SEPARATOR: char = '/';

/// Character that separate the path from the query
pub const QUERY_CHARACTER: char = '?';

/// A UTF-8 path divided into three components:
/// - Mount point
/// - Resource location
/// - Query
///
/// # Examples
///
/// - //assets/textures/texture.png?mip=0
/// - /mountpoint/assets/textures/texture.png
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Path {
    str: String,
    mount_point: Option<Range<usize>>,
    path: Range<usize>,
    query: Option<Range<usize>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParseError {
    InvalidMountPoint,
}

impl Path {
    pub fn parse(str: &str) -> Result<Path, ParseError> {
        if str.chars().next().unwrap_or_default() != PATH_SEPARATOR {
            return Err(ParseError::InvalidMountPoint);
        }

        let mut string = str.to_string();

        let mount_point = {
            let end_mount_point = str
                .chars()
                .enumerate()
                .position(|(i, c)| i != 0 && c == PATH_SEPARATOR);
            end_mount_point.map(|end| 1..end)
        }
        .ok_or(ParseError::InvalidMountPoint)?;

        let query = {
            let start_query = str.chars().position(|c| c == QUERY_CHARACTER);
            start_query.map(|start| start + 1..str.len())
        };

        let mut path = if let Some(query) = &query {
            mount_point.end + 1..query.start - 1
        } else {
            mount_point.end + 1..str.len()
        };

        // Remove trailing slash
        if string[path.clone()].chars().last().unwrap() == PATH_SEPARATOR {
            string.remove(path.end - 1);
            path.end -= 1;
        }

        Ok(Path {
            str: string,
            mount_point: if mount_point.is_empty() {
                None
            } else {
                Some(mount_point)
            },
            path,
            query,
        })
    }

    pub fn from_mount_point_and_path(mount_point: &str, path: &str) -> Path {
        let str = format!("{PATH_SEPARATOR}{mount_point}{PATH_SEPARATOR}{path}");
        Path::parse(&str).unwrap()
    }

    /// Extends the path with a new path component.
    ///
    /// # Examples
    ///
    /// ```
    /// use ze_filesystem::path::Path;
    ///
    /// let mut path = Path::parse("/m/path/to/").unwrap();
    /// path.push("new_file.txt");
    /// assert_eq!(path.path(), "path/to/new_file.txt");
    /// ```
    pub fn push(&mut self, other: impl ToString) {
        let mut str = other.to_string();
        if str.chars().next().unwrap_or_default() != PATH_SEPARATOR {
            str.insert(0, PATH_SEPARATOR);
        }

        self.str.insert_str(self.path.end, &str);
        self.path.end += str.len();

        if let Some(query) = &mut self.query {
            query.start += str.len();
            query.end += str.len();
        }
    }

    /// Like `push`, but returns a new path instead of mutating the current one.
    ///
    /// # Examples
    ///
    /// ```
    /// use ze_filesystem::path::Path;
    ///
    /// let path = Path::parse("/m/path/to/").unwrap();
    /// let path = path.join("new_file.txt");
    /// assert_eq!(path.path(), "path/to/new_file.txt");
    /// ```
    pub fn join(&self, other: impl ToString) -> Path {
        let mut path = self.clone();
        path.push(other);
        path
    }

    /// Override the path with a new path component.
    ///
    /// # Examples
    ///
    /// ```
    /// use ze_filesystem::path::Path;
    ///
    /// let mut path = Path::parse("/m/path/to/file.txt").unwrap();
    /// path.set_path("path/to/file2.txt");
    /// assert_eq!(path.path(), "path/to/file2.txt");
    /// ```
    pub fn set_path(&mut self, new: impl ToString) {
        let new = new.to_string();
        let old = self.path.clone();
        self.str.replace_range(old.clone(), &new);
        self.path = old.start..old.start + new.len();

        if let Some(query) = &mut self.query {
            let len = query.len();
            query.start = self.path.end + 1;
            query.end = query.start + len;
        }
    }

    pub fn path_segments(&self) -> std::str::Split<'_, char> {
        self.path().split(PATH_SEPARATOR)
    }

    pub fn as_str(&self) -> &str {
        &self.str
    }

    pub fn mount_point(&self) -> Option<&str> {
        self.mount_point
            .as_ref()
            .map(|mount_point| &self.str[mount_point.clone()])
    }

    pub fn path(&self) -> &str {
        &self.str[self.path.clone()]
    }

    pub fn query(&self) -> Option<&str> {
        self.query.as_ref().map(|range| &self.str[range.clone()])
    }
}

impl Deref for Path {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.str
    }
}

impl DerefMut for Path {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.str
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.str)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse() {
        let path = super::Path::parse("//assets/textures/texture.png?mip=0").unwrap();
        assert!(path.mount_point().is_none());
        assert_eq!(path.path(), "assets/textures/texture.png");
        assert_eq!(path.query().unwrap(), "mip=0");

        let path = super::Path::parse("/mntpnt/assets/textures/texture.png").unwrap();
        assert_eq!(path.mount_point(), Some("mntpnt"));
        assert_eq!(path.path(), "assets/textures/texture.png");
        assert!(path.query().is_none());

        assert_eq!(
            super::Path::parse("mntpnt/assets/textures/texture.png"),
            Err(super::ParseError::InvalidMountPoint)
        );

        assert_eq!(
            super::Path::parse("/mntpnt"),
            Err(super::ParseError::InvalidMountPoint)
        );
    }

    #[test]
    fn from_mount_point_and_path() {
        let path = super::Path::from_mount_point_and_path("mnt", "assets/textures/texture.png");
        assert_eq!(path.mount_point(), Some("mnt"));
        assert_eq!(path.path(), "assets/textures/texture.png");
    }

    #[test]
    fn push() {
        let mut path = super::Path::from_mount_point_and_path("mnt", "assets/textures?mip=0");
        path.push("texture.png");

        assert_eq!(path.mount_point(), Some("mnt"));
        assert_eq!(path.path(), "assets/textures/texture.png");
        assert_eq!(path.query().unwrap(), "mip=0");
    }
}
