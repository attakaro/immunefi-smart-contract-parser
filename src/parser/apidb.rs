use std::collections::HashMap;
use std::error::Error;

use tokio::fs;

// hashmap<name, (key, api url)>

#[derive(Debug)]
pub struct ApiDB {
    pub db: HashMap<String, (String, String)>
}

impl ApiDB {

    // init 

    pub fn new() -> Self {
        Self { 
            db: HashMap::new() 
        }
    }

    // read from db

    pub async fn read(&mut self) -> Result<(), Box<dyn Error>> {
        let json_str = fs::read_to_string("./keys.json").await?;
        let apis: HashMap<String, (String, String)> = serde_json::from_str(&json_str)?;
        self.db = apis;
        Ok(())
    }

    // write to db

    async fn write(&self) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string_pretty(&self.db)?;
        fs::write("./keys.json", json).await?;
        Ok(())
    }

    // change api key

    pub async fn change_api_key(&mut self, name: &str, new_key: &str) -> Result<(), Box<dyn Error>> {
        match self.db.get_mut(name) {
            Some(val) => { 
                val.0 = new_key.to_owned(); // val.0 is old key
                self.write().await?;
            },
            None => {
                eprintln!("No such name \"{}\" in database!", name);
            }
        }
        Ok(())
    }

    // change api url

    pub async fn change_api_url(&mut self, name: &str, new_url: &str) -> Result<(), Box<dyn Error>> {
        match self.db.get_mut(name) {
            Some(val) => { 
                val.1 = new_url.to_owned(); // val.1 is old api url
                self.write().await?;
            },
            None => {
                eprintln!("No such name \"{}\" in database!", name);
            }
        }
        Ok(())
    }

    // add new chain to db

    pub async fn add_new_api(&mut self, name: &str, key: &str, api: &str) -> Result<(), Box<dyn Error>> {
        self.db.insert(name.to_owned(), (key.to_owned(), api.to_owned()));
        self.write().await?;
        Ok(())
    }

    // remove chain from db

    pub async fn remove_api(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        self.db.remove(name);
        self.write().await?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn database_functions_test() -> Result<(), Box<dyn Error>> {
        let mut db = ApiDB::new();
        db.read().await?;

        // add new api test
        db.add_new_api("test", "test_key", "test_api").await?;
        db.read().await?;
        assert_eq!(db.db.get("test"), Some(&("test_key".to_owned(), "test_api".to_owned())));

        // change api key test
        db.change_api_key("test", "new_key").await?;
        db.read().await?;
        assert_eq!(db.db.get("test").unwrap().0, "new_key".to_owned());
        
        // change api url test
        db.change_api_url("test", "new_api_url").await?;
        db.read().await?;
        assert_eq!(db.db.get("test").unwrap().1, "new_api_url".to_owned());

        // remove api test
        db.remove_api("test").await?;
        db.read().await?;
        assert_eq!(db.db.get("test"), None);

        Ok(())
    }
}