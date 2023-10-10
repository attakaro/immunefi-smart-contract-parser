use std::error::Error;
use clap::{Arg, Command};

mod parser;
use parser::ApiDB;
use parser::Parser;
use parser::ParserMode;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    // init api database

    let mut db = ApiDB::new();

    db.read().await?;

    // subcommands list

    let matches = Command::new("iscp")
        .about("$$$ Immunefi smart contract parser $$$")
        .version("0.1.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .author("attakaro")
        .subcommand(
            Command::new("parse")
                .about("parse one smart contract using direct url")
                .arg(
                    Arg::new("url")
                        .help("smart contract direct url e.g. \"https://etherscan.io/0xFFFFFFFFFFFFFFFFFFFFFFFF\"")
                )
        )
        .subcommand(
            Command::new("parse_imm")
                .about("parse smart contracts from immunefi bounty page(github links not included for now)")
                .arg(
                    Arg::new("immunefi url")
                        .help("immunefi bounty link e.g. \"https://immunefi.com/bounty/project/\"")
                )
                .arg(
                    Arg::new("folder name")
                        .help("sets name of created folder with all parsed contracts")
                )
                .arg(
                    Arg::new("concurrent requests limit")
                        .help("how many api requests at the same time")
                )
        )
        .subcommand(
            Command::new("change_api_key")
                .about("change api key in api database")
                .arg(
                    Arg::new("name")
                        .help("chain name(used as db key)")
                )
                .arg(
                    Arg::new("new key")
                        .help("your new api key")
                )
        )
        .subcommand(
            Command::new("change_api_url")
                .about("change api url in api database")
                .arg(
                    Arg::new("name")
                        .help("chain name(used as db key)")
                )
                .arg(
                    Arg::new("new url")
                        .help("your new api url")
                )
        )
        .subcommand(
            Command::new("add_api")
                .about("add new api to database")
                .arg(
                    Arg::new("name")
                        .help("chain name(as substring of direct contract url e.g. \"etherscan\" name and \"https://etherscan.io/0xfffffff\" url")
                )
                .arg(
                    Arg::new("key")
                        .help("your api key")
                )
                .arg(
                    Arg::new("api url")
                        .help("your api url e.g. \"https://api.etherscan.io\"")
                )
        )
        .subcommand(
            Command::new("remove_api")
                .about("remove api from database")
                .arg(
                    Arg::new("name")
                        .help("chain name(used as db key)")
                )
        )
        .get_matches();

    // matching subcommands

    match matches.subcommand() {
        Some(("parse", arg)) => {
            if arg.contains_id("url") {
                let mode = ParserMode::Single;
                let url = arg.get_one::<String>("url").unwrap();
                println!("\n### Parsing started! ###\n");
                Parser::parse_contract(url, &db, &mode, false).await?;
                println!("\n### Parsing finished! ###");
                Ok(())
            } else {
                Err("not all args were provided".into())
            }
        }
        Some(("parse_imm", args)) => {
            if args.contains_id("immunefi url") 
            && args.contains_id("folder name") 
            && args.contains_id("concurrent requests limit")  {
                let url = args.get_one::<String>("immunefi url").unwrap();
                let folder_name = args.get_one::<String>("folder name").unwrap();
                let limit = args.get_one::<String>("concurrent requests limit").unwrap().parse::<usize>()?;
                println!("\n### Parsing started! ###\n");
                Parser::immunefi_traverse(url, &db, false, folder_name, limit).await?;
                println!("\n### Parsing finished! ###");
                Ok(())
            } else {
                Err("not all args were provided".into())
            }
        }
        Some(("change_api_key", args)) => {
            if args.contains_id("name") 
            && args.contains_id("new key") {
                let name = args.get_one::<String>("name").unwrap();
                let new_key = args.get_one::<String>("new key").unwrap();
                db.change_api_key(name, new_key).await?;
                println!("### Database updated! ###\n");
                println!("changed api key to \"{}\" for name \"{}\"", new_key, name);
                Ok(())
            } else {
                Err("not all args were provided".into())
            }
        }
        Some(("change_api_url", args)) => {
            if args.contains_id("name") 
            && args.contains_id("new url") {
                let name = args.get_one::<String>("name").unwrap();
                let new_url = args.get_one::<String>("new url").unwrap();
                db.change_api_url(name, new_url).await?;
                println!("### Database updated! ###\n");
                println!("changed api url to \"{}\" for name \"{}\"", new_url, name);
                Ok(())
            } else {
                Err("not all args were provided".into())
            }
        }
        Some(("add_api", args)) => {
            if args.contains_id("name") 
            && args.contains_id("key") 
            && args.contains_id("api url") {
                let name = args.get_one::<String>("name").unwrap();
                let key = args.get_one::<String>("key").unwrap();
                let api_url = args.get_one::<String>("api url").unwrap();
                db.add_new_api(name, key, api_url).await?;
                println!("### Database updated! ###\n");
                println!("added new name: {},\nkey: {},\napi url: {}", name, key, api_url);
                Ok(())
            } else {
                Err("not all args were provided".into())
            }
        }
        Some(("remove_api", arg)) => {
            if arg.contains_id("name") {
                let name = arg.get_one::<String>("name").unwrap();
                db.remove_api(name).await?;
                println!("### Database updated! ###\n");
                println!("removed {} api from database", name);
                Ok(())
            } else {
                Err("not all args were provided".into())
            }
        }
        _ => unreachable!()
    } 
}