/*  dir-obj-rs: the requirements tracking tool made for developers
    Copyright (C) 2017  Garrett Berg <@vitiral, vitiral@gmail.com>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the Lesser GNU General Public License as published 
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the Lesser GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/
//! Simple package to mimick a directory structure in ram
//! Provides bindings (through feature flags) for conversion to/from
//! various libraries

// traits
use std::io::{Read, Write};

use std::io;
use std::fs;
use std::ffi::OsString;
use std::collections::{HashMap};
use std::collections::hash_map;
use std::path::{Path};

/// representation of a directory
#[derive(Debug, PartialEq)]
pub struct Dir {
    items: HashMap<OsString, Entry>,
}

/// representation of a file
#[derive(Debug, PartialEq)]
pub struct File {
    bytes: Vec<u8>,
}

/// possible entries in a directory
#[derive(Debug, PartialEq)]
pub enum Entry {
    File(File),
    Dir(Dir),
}

impl Entry {
    pub fn dump(&self, path: &Path) -> io::Result<()> {
        match *self {
            Entry::File(ref f) => f.dump(path),
            Entry::Dir(ref d) => d.dump(path),
        }
    }
}


impl File {
    pub fn new(bytes: Vec<u8>) -> File {
        File {
            bytes: bytes,
        }
    }

    pub fn load(path: &Path) -> io::Result<File> {
        //println!("loading file: {}", path.display());
        let mut f = fs::File::open(path)?;
        let mut bytes: Vec<u8> = Vec::new();
        f.read_to_end(&mut bytes)?;
        Ok(File::new(bytes))
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn dump(&self, path: &Path) -> io::Result<()> {
        let mut f = fs::File::create(path)?;
        f.write_all(&self.bytes)
    }

}

impl Dir {
    pub fn new() -> Dir {
        Dir {
            items: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> io::Result<Dir> {
        //println!("loading dir : {}", path.display());
        let mut dir = Dir::new();
        let read_dir = fs::read_dir(path)?;
        for e in read_dir {
            let entry = e?;
            let ftype = entry.file_type()?;
            if ftype.is_dir() {
                let subdir = Dir::load(&entry.path())?;
                dir.items.insert(entry.file_name(), Entry::Dir(subdir));
            } else if ftype.is_file() {
                let file = File::load(&entry.path())?;
                dir.items.insert(entry.file_name(), Entry::File(file));
            } else {
                return Err(io::ErrorKind::Other.into());
            }
        }
        Ok(dir)
    }

    pub fn dump(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)?;
        for (name, entry) in self.items.iter() {
            let epath = path.join(name);
            entry.dump(&epath)?;
        }
        Ok(())
    }

    pub fn add_file(&mut self, name: OsString, file: File) -> io::Result<()> {
        if self.items.contains_key(&name) {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        self.items.insert(name, Entry::File(file));
        Ok(())
    }

    pub fn add_dir(&mut self, name: OsString, dir: Dir) -> io::Result<()> {
        if self.items.contains_key(&name) {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        self.items.insert(name, Entry::Dir(dir));
        Ok(())
    }

    pub fn entries(&self) -> hash_map::Iter<OsString, Entry> {
        self.items.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::str;
    use std::path::PathBuf;

    fn validate_dir(level: usize, dir: &Dir) -> Result<(), String> {
        for (n, entry) in dir.entries() {
            let name = n.to_str().unwrap();
            let (name_prefix, last_c) = name.split_at(name.len()-1);
            last_c.parse::<usize>().expect(&format!("last_c: {}, invalid name: {}", 
                                                    last_c, name));
            match *entry {
                Entry::File(ref file) => {
                    if name_prefix != format!("file{}-", level) {
                        return Err(format!("lvl {} file {} has invalid name", level, name));
                    }
                    let data = str::from_utf8(file.bytes()).unwrap();
                    if data.trim() != name {
                        return Err(format!("lvl {} file {} has invalid data: \"{}\"", 
                                           level, name, data));
                    }
                }
                Entry::Dir(ref dir) => {
                    if name_prefix != format!("dir{}-", level) {
                        return Err(format!("invalid dir name at level {}: {}", level, name));
                    }
                    //println!("validating dir name: {}", name);
                    validate_dir(level + 1, dir)?;
                }
            }
        }
        Ok(())
    }

    fn count_entries(dir: &Dir) -> usize {
        let mut count = 0;
        for (_, entry) in dir.entries() {
            count += 1;
            if let &Entry::Dir(ref d) = entry {
                count += count_entries(d);
            }
        }
        count
    }

    #[test]
    /// really basic test to just load a directory, validate it, 
    /// save it to a new folder and reload + validate it
    fn test() {
        let cwd = env::current_dir().unwrap();
        let test_dir: PathBuf = cwd.join(PathBuf::from(
            file!()).parent().unwrap().to_path_buf());
        let data_dir: PathBuf = test_dir.join(PathBuf::from("data"));

        println!("loading data dir: {}", data_dir.display());
        let dir = Dir::load(&data_dir).unwrap();
        println!("validating data dir: {}", data_dir.display());
        assert_eq!(count_entries(&dir), 10);
        validate_dir(0, &dir).expect("data dir");
        let tmp: PathBuf = test_dir.join(PathBuf::from("test_out_dir"));

        println!("dumping to: {}", tmp.display());
        dir.dump(&tmp).expect("couldn't dump dir");
        let result = Dir::load(&tmp).unwrap();
        assert_eq!(result, dir);
        fs::remove_dir_all(tmp).expect("couldn't remove");
    }
}
