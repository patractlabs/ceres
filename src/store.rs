//! Storage implementation
use ceres_executor::Memory;
use ceres_runtime::{Metadata, Runtime};
use ceres_support::traits;
use etc::{Etc, FileSystem, Meta};
use sled::Db;
use std::{fs, path::PathBuf, process};

/// A ceres storage implementation using sled
#[derive(Clone)]
pub struct Storage {
    pub db: Db,
    memory: Vec<Memory>,
    frame: Vec<Vec<u8>>,
}

impl traits::Storage for Storage {
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) -> Option<Vec<u8>> {
        self.db.insert(key, value).ok()?.map(|v| v.to_vec())
    }

    fn remove(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.db.remove(key).ok()?.map(|v| v.to_vec())
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.db.get(key).ok()?.map(|v| v.to_vec())
    }
}

impl traits::Frame for Storage {
    /// Current id
    fn id(&self) -> usize {
        self.frame.len()
    }

    /// active frame
    fn active(&self) -> Option<Vec<u8>> {
        self.frame.last().map(|v| v.clone())
    }

    /// Pop frame
    fn pop_frame(&mut self) -> Option<Vec<u8>> {
        self.frame.pop()
    }

    /// Push frame
    fn push_frame(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.frame.push(key.to_vec());
        Some(key.to_vec())
    }
}

impl traits::State<Memory> for Storage {
    fn memory(&self) -> Option<Memory> {
        Some(self.memory[self.memory.len() - 1].clone())
    }

    fn memory_mut(&mut self) -> Option<&mut Memory> {
        self.memory.last_mut()
    }
    /// Get memory mut
    fn pop_memory(&mut self) -> Option<Memory> {
        self.memory.pop()
    }

    /// Get memory mut
    fn push_memory(&mut self, memory: Memory) -> Option<()> {
        self.memory.push(memory);
        Some(())
    }
}

impl traits::Cache<Memory> for Storage {}

impl Storage {
    fn quit() {
        println!(
            "The following required arguments were not provided: \n\t\t\
             <*.contract | name | code-hash>"
        );
        process::exit(1);
    }

    /// New storage
    pub fn new() -> crate::Result<Self> {
        let etc = Etc::new(&dirs::home_dir().ok_or("Could not find home dir")?)?;

        Ok(Self {
            db: sled::open(etc.open(".ceres/contracts")?.real_path()?)?,
            memory: Vec::new(),
            frame: Vec::new(),
        })
    }

    /// Contract instance
    ///
    /// * From path of `*.contract`
    /// * From name of `*.contract`
    /// * From code_hash of `*.contract`
    pub fn rt(&mut self, contract: &str) -> crate::Result<Runtime> {
        let if_path = PathBuf::from(contract);
        let cache = self.clone();
        Ok(if if_path.exists() {
            let source = fs::read(if_path)?;
            let rt = Runtime::from_contract(&source, cache, Some(ceres_ri::Instance))?;
            self.db.insert(
                &rt.metadata.contract.name,
                bincode::serialize(&rt.metadata.clone())?,
            )?;
            rt
        } else if let Ok(Some(contract)) = if contract.is_empty() {
            let mut recent = None;
            for c in self.db.iter() {
                let (k, v) = c?;
                if k.len() != 32 {
                    recent = Some(Ok(Some(v)));
                    break;
                }
            }

            if let Some(r) = recent {
                r
            } else {
                return Err(crate::Error::ParseContractFailed(
                    "Get recent contract failed".to_string(),
                ));
            }
        } else {
            self.db.get(contract.as_bytes())
        } {
            Runtime::from_metadata(
                bincode::deserialize::<Metadata>(&contract)?,
                cache,
                Some(ceres_ri::Instance),
            )?
        } else {
            Self::quit();

            // NOTE:
            //
            // Unreachable error
            return Err(crate::Error::ParseContractFailed(contract.to_string()));
        })
    }
}
