mod apidb;
pub use apidb::*;

mod models;
pub use models::*;

use std::error::Error;
use tokio::fs;
use futures::{stream, StreamExt};
use scraper::{Html, Selector};
use serde_json::Value;
use regex::Regex;

pub struct Parser;

impl Parser {

    // traversing smart contracts listed on immunefi project page

    pub async fn immunefi_traverse(
        url: &str, 
        api: &ApiDB, 
        open_zeppelin: bool, 
        folder_name: &str, 
        limit: usize
    ) -> Result<(), Box<dyn Error>> {

        let response = reqwest::get(url).await?.text().await?;
        let document = Html::parse_document(&response);
        let contract_name_selector = 
            Selector::parse("section.mb-12:nth-child(3) > div:nth-child(2)")?;
        let elems = document
            .select(&contract_name_selector)
            .collect::<Vec<_>>();
        if elems.is_empty() {
            return Err("wrong immunefi page or selector".into())
        }
        let urls = elems[0].text()
                        .filter(|&url| {
                            api.db.keys().any(|name| url.contains(name))
                        }).collect::<Vec<_>>();
        
        // concurrent option in case better api plan is provided(for free api set limit to 2)

        stream::iter(urls).for_each_concurrent(limit, |url| async move {
            let mode = 
                ParserMode::Immunefi(folder_name.to_owned());
            Parser::parse_contract(url, api, &mode, open_zeppelin)
                .await
                .unwrap_or_else(|why|{
                    eprintln!("error parsing contract {} \n {}", url, why)
                })
        }).await;
        
        Ok(())
    }

    // concat url parts

    fn get_api_url(url: &str, contract_address: &str, apis: &ApiDB) -> String {
        let mut key = String::new();
        let mut api = String::new();
    
        for (name, key_and_api) in &apis.db {
            if url.contains(name) {
                (key, api) = key_and_api.to_owned();
                break
            }
        }

        // workaround because etherscan is a subsrting for both etherscan.io and optimistic.etherscan.io
        if url.contains("optimistic") {
            (key, api) = apis.db.get("optimistic").unwrap().to_owned();
        }
    
        let api_url = format!("{}/api?module=contract&action=getsourcecode&address={}&apikey={}", 
            api, 
            contract_address, 
            key
        );

        api_url
    }

    // get contract address from url

    async fn get_contract_address(url: &str) -> Result<String, Box<dyn Error>> {
        let addr_pattern = Regex::new(r"0x[0-9a-fA-F]{40}")?;
        match addr_pattern.find(url) {
            Some(address) => Ok(address.as_str().to_owned()),
            None => {
                let address = Parser::scrape_contract_address(url).await?;
                Ok(address)
            }
        }
    }
    
    // scrape contract address if no regex match

    async fn scrape_contract_address(url: &str) -> Result<String, Box<dyn Error>> {
        let response = reqwest::get(url).await?;
        let body = response.text().await?;
        let document = Html::parse_document(&body);
        let contract_name_selector = Selector::parse("#mainaddress")?;
        let elems = document.select(&contract_name_selector).collect::<Vec<_>>();
        if elems.is_empty() {
            return Err("wrong smart contract page or selector".into())
        }
        let address = elems[0].text().collect::<Vec<_>>()[0].trim().to_owned();
        Ok(address)
    }

    // api request

    async fn get_contract_data(url: &str, contract_address: &str, api: &ApiDB) -> Result<ContractData, Box::<dyn Error>> {
        let url = Parser::get_api_url(url, contract_address, api);
        let response = reqwest::get(url).await?;
        let body = response.text().await?;
        let json: serde_json::Value = serde_json::from_str(&body)?;
        let (name, code) = (
            json["result"][0]["ContractName"].as_str()
                .unwrap_or("couldn't convert SourceCode to string"),
            json["result"][0]["SourceCode"].as_str()
                .unwrap_or("couldn't convert SourceCode to string"));
        let data = ContractData {
            name: name.to_owned(),
            code: code.to_owned()
        };
        Ok(data)
    }

    // get directory of splitted contract from path

    fn get_dir_of_splitted_contract(file_path: &str) -> &str {
        let chars: Vec<char> = file_path.chars().collect();
        let start = 0;
        let mut end = chars.len() - 1;
        while chars[end] != '/' {
            end -= 1;
        }
        &file_path[start..end]
    }

    // check if source code is splitted into separate files 

    fn get_contract_type(contract: &ContractData) -> ContractType {
        if contract.code.starts_with('{') {
            ContractType::Splitted
        } else {
            ContractType::United
        }
    }
    
    // parsing

    pub async fn parse_contract(
        url: &str, 
        api: &ApiDB, 
        mode: &ParserMode, 
        open_zeppelin: bool 
    ) -> Result<(), Box<dyn Error>> {

        // init

        let contract_address = Parser::get_contract_address(url).await?;
        let mut contract_data = Parser::get_contract_data(url, &contract_address, api).await?;
        let contract_type = Parser::get_contract_type(&contract_data);

        // parsing

        match contract_type {
            ContractType::Splitted => {

                // idk why etherscan(or maybe other chains too) sometimes sends json with double curly braces {{}}
                if contract_data.code.starts_with("{{") {
                    contract_data.code.remove(0); 
                    contract_data.code.pop();
                }

                // parse the json data
                let json: Value = serde_json::from_str(&contract_data.code)?;
                
                // access the contract sources
                match json["sources"].as_object() {
                    Some(sources) => {
                        for (path, source_info) in sources.iter() {
                            // ignore @openzeppelin libraries and import.sol files
                            match path {
                                _ if path.contains("@openzeppelin") => if !open_zeppelin {continue},
                                _ if path.contains("/import.sol") => continue,
                                _ => {}
                            }
                            // accessing content(source code)
                            match source_info["content"].as_str() {
                                Some(source_content) => {
                                    Parser::save_splitted_contract(mode, &contract_address, path, source_content).await?
                                }
                                None => eprintln!("Couldn't access \"content\" field in returned JSON of contract \"{}\" {}", contract_data.name, contract_address)
                            }
                        }
                    }
                    None => eprintln!("Couldn't access \"sources\" field in returned JSON of contract \"{}\" {}", contract_data.name, contract_address)
                } 
            },
            ContractType::United => {
                Parser::save_united_contract(&contract_data, mode, &contract_address).await?
            }
        }
        Ok(())
    }

    async fn save_splitted_contract( 
        mode: &ParserMode, 
        contract_address: &str,
        path: &str,
        source_content: &str
    ) -> Result<(), Box<dyn Error>> {

        // get directory part of the contract

        let dir_part = Parser::get_dir_of_splitted_contract(path);

        // create path with immunefi name at the beginning in immunefi mode 
        // or with addr in single mode

        let file_path = match mode {
            ParserMode::Immunefi(folder_name) => format!("{}/{}/{}", folder_name, contract_address, path),
            ParserMode::Single => format!("{}/{}", contract_address, path) 
        };

        // if directory doesn't exist, create new, otherwise overwrite

        let dir = fs::metadata(&file_path).await;
        match dir {
            Ok(_) => fs::write(&file_path, source_content).await?,
            Err(_) => {
                let dir = match mode {
                    ParserMode::Immunefi(folder_name) => format!("{}/{}/{}", folder_name, contract_address, dir_part),
                    ParserMode::Single => format!("{}/{}", contract_address, dir_part) 
                };
                fs::create_dir_all(dir).await?;
                fs::write(&file_path, source_content).await?
            }
        }

        println!("{} has been created!", file_path);

        Ok(())
    }

    async fn save_united_contract(
        contract_data: &ContractData,
        mode: &ParserMode, 
        contract_address: &str,
    ) -> Result<(), Box<dyn Error>> {

        // create path with immunefi name at the beginning in immunefi mode 
        // or with addr in single mode

        let file_path = match mode {
            ParserMode::Immunefi(folder_name) => format!("{}/{}/{}.sol", folder_name, contract_address, contract_data.name),
            ParserMode::Single => format!("{}/{}.sol", contract_address, contract_data.name)
        };

        // if directory doesn't exist, create new, otherwise overwrite

        let dir = fs::metadata(&file_path).await;
        match dir {
            Ok(_) => fs::write(&file_path, &contract_data.code).await?,
            Err(_) => {
                let new_dir = match mode {
                    ParserMode::Immunefi(folder_name) => format!("{}/{}", folder_name, contract_address),
                    ParserMode::Single => contract_address.to_owned()
                };
                fs::create_dir_all(new_dir).await?;
                fs::write(&file_path, &contract_data.code).await?;
            }
        }

        println!("{} has been created!", file_path);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_contract_address_test() -> Result<(), Box<dyn Error>> {
        let url = "https://etherscan.io/address/0xdac17f958d2ee523a2206206994597c13d831ec7";
        let address = Parser::get_contract_address(url).await?;
        assert_eq!(address, "0xdac17f958d2ee523a2206206994597c13d831ec7");
        Ok(())
    }

    #[tokio::test]
    async fn parse_single_contract_test() -> Result<(), Box<dyn Error>> {
        let mut db = ApiDB::new();
        db.read().await?;
        let mode = ParserMode::Single;
        let url = "https://etherscan.io/address/0xdac17f958d2ee523a2206206994597c13d831ec7";
        Parser::parse_contract(url, &db, &mode, false).await?;
        Ok(())
    }

    #[tokio::test]
    async fn parse_from_immunefi_test() -> Result<(), Box<dyn Error>> {
        let mut db = ApiDB::new();
        db.read().await?;
        let url = "https://immunefi.com/bounty/sushiswap/";
        let folder_name = "sushi swap";
        Parser::immunefi_traverse(url, &db, false, folder_name, 2).await?;
        Ok(())
    }
}