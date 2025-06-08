use std::{
    fs::{self, create_dir, File},
    io::{Error, Write},
    path::{Path, PathBuf},
};

pub trait GameSave {
    fn set_title(&mut self, _title: String) {}

    fn load(&self) -> Result<Vec<u8>, Error> {
        Ok(vec![])
    }

    fn save(&self, _ram: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

pub struct Fake;

impl GameSave for Fake {}

pub struct FileSaver {
    save_path: PathBuf,
}

impl FileSaver {
    pub fn new() -> Result<Self, Error> {
        let save_path = Path::new("save");
        if !save_path.is_dir() {
            create_dir("save")?;
        }

        let save_path = save_path.to_path_buf();

        Ok(Self { save_path })
    }
}

impl GameSave for FileSaver {
    fn set_title(&mut self, title: String) {
        self.save_path = self.save_path.join(title + ".sav");
    }

    fn load(&self) -> Result<Vec<u8>, Error> {
        fs::read(self.save_path.clone())
    }

    fn save(&self, ram: &[u8]) -> Result<(), Error> {
        let mut file = File::create(self.save_path.clone())?;
        file.write_all(ram)?;

        Ok(())
    }
}
