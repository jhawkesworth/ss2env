/* ss2env
 *
 * A small wrapper program that you can use neosmart's securestore secrets with jet.
 *
 * It works like this:
 * 1/ read all the secrets in the -s/--store arg path_to_secrets.json file and
 * 2/ using the key in the -k/--key arg path_to_secrets.key (or default of ~/.securestore/secrets.key)
    , decrypt the secrets in memory
 * 3/ turn all the secrets into environment variables (changes : to _ )
 * 4/ run jetp passing the secret environment and any jetp arguments.

 * In the interests of keeping the command line clutter to a minimum there are defaults for -s/--store and -k/--key as follows
   -k/--key default is ~/.securestore/secrets.key
   -s/--store default is to look for a file called 'secrets.json' in the same dir as the jetp -i inventory path.


 * Remember that unlike other tools, jetp only makes secrets available to template module
 * (and after 0.2 module parameters too)
 * any secrets, once passed are all prefixed ENV_

 * so if your secret is called db:postgres it will be passed TO jet as db_postgres
 * and to use it in your template you will need to enter
 * {{ ENV_db_postgres }}

 * For more information on Jetporch see
 *   https://www.jetporch.com/
 *
 * For specific information about handling secrets when using Jetporch see
 *   https://www.jetporch.com/playbooks/managing-secrets

 * For more information on using securestore's client program, ssclient, see
 *   https://crates.io/crates/ssclient

 * For details of the securestore rust API see
 *   https://github.com/neosmart/securestore-rs/blob/master/securestore/src/lib.rs
 *
*/

use securestore::{KeySource, SecretsManager};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};
use std::ffi::OsStr;

// Config struct holds setting taken from command line args
#[derive(Debug)]
struct Config {
    secret_file_path: String,
    secret_key_path: String,
    target_command_args: Vec<String>,
}

impl Config {
    // parse cli args into Config
    fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {

        args.next();

        let mut secret_key: Option<String> = Option::from(String::from("~/.securestore/secrets.key"));
        let mut secret_store: Option<String> = Option::None; // default calculated later after args are parsed.
        let mut target_command_args: Vec<String> = Vec::new(); // will hold program and args to run with environment containing secrets

        match args.next() {
            Some(arg) => {
                if (arg == "--store") || (arg == "-s") {
                    match args.next() {
                        Some(store_arg) => secret_store = Option::from(store_arg),
                        _ => {},
                    }
                } else if (arg == "--key") || (arg == "-k" ) {
                    match args.next() {
                        Some(key_arg) => secret_key = Option::from(key_arg),
                        _ => {},
                    }
                } else {
                    target_command_args.push(arg);
                }
            },
            _ => {},
        };

        // yep do the same thing again to capture the other arg
        match args.next() {
            Some(arg) => {
                if (arg == "--store") || (arg == "-s") {
                    match args.next() {
                        Some(store_arg) => secret_store = Option::from(store_arg),
                        _ => {},
                    }
                } else if (arg == "--key") || (arg == "-k" ) {
                    match args.next() {
                        Some(key_arg) => secret_key = Option::from(key_arg),
                        _ => {},
                    }
                } else {
                    target_command_args.push(arg);
                }
            },
            _ => {},
        };

        // slurp the rest of the args from the command line, should be jetp and all its args
        for arg in args {
            target_command_args.push(arg);
        }

        if secret_store.is_none() {
            // look for the arg after -i in the target command line
            //
            let tci = target_command_args.iter();
            let mut getpath = false;
            for t_arg in tci {
                if getpath == true {
                    let inventory_path = Path::new(t_arg);
                    // get parent dir of the -i path
                    let inv_path_base = inventory_path.parent().unwrap_or(Path::new("."));
                    let mut conventional_secret_store = PathBuf::from(inv_path_base);
                    // add secrets.json to the path (or . if not found)
                    conventional_secret_store.push(Path::new("secrets.json"));
                    secret_store = Option::Some(String::from(conventional_secret_store.to_str().unwrap()));

                    break;
                }
                if t_arg == "-i" {
                    getpath = true; // retrieve the path next time through the loop
                }

            }
        }

        // target command must contain at least one arg (path to jetp)

        let args_len = target_command_args.len();
        if args_len < 1 {
            return Err("not enough arguments ");
        }

        // try to read the key
        let key_read = std::fs::read(&secret_key.as_ref().unwrap());
        if key_read.is_err() {
            println!("Could not read secret key at [{}].  Either supply -k/--key arg or put your secrets.key in ~/.securestore/secrets.key", &secret_key.clone().unwrap());
            return Err("could not read key file. ")
        }

        // make sure we have a path for the secret store
        if secret_store.is_none() {
            return Err("no secret file specified or found adjacent to jetp -i inventory directory. ")
        }

        // check we can read the secret store file
        let secret_read = std::fs::read(&secret_store.clone().unwrap());
        if secret_read.is_err() {
            println!("Could not read secret file at [{}].  Either supply -s/--store arg or put secret.json in the same dir as your -i inventory", &secret_store.clone().unwrap());
            return Err("could not read secret file. ");
        }

        // check the target program is called 'jetp' to make it slightly harder
        // to exfiltrate secrets
        let target_file_path = Path::new(&target_command_args[0]);
        let target_file_stem = target_file_path.file_stem();
        if ! target_file_stem.eq(&Some(OsStr::new("jetp"))) {
            return Err("target command must be 'jetp'. ");
        }

        let secret_file_path = secret_store.unwrap().clone();
        let secret_key_path = secret_key.unwrap().clone();

        Ok(Config {
            secret_file_path,
            secret_key_path,
            target_command_args,
        })
    }
}

fn main() {
    let config = Config::build(env::args()).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {err}\n Usage: ss2env [--store path_to_secret.json] [--key path_to_secret.key] path_to_jetp [arguments_to_pass_to_jetp..]");
        process::exit(1);
    });

    //println!("configuration is {:?}", config);

    let keyfile = Path::new(&config.secret_key_path);
    let secrets_load_result = SecretsManager::load(&config.secret_file_path, KeySource::File(keyfile));
    if let Ok(secrets) = secrets_load_result {

        let mut secret_count = 0;
        let mut secrets_as_env_vars: HashMap<String, String> = HashMap::new();

        for key in secrets.keys() {
            if let Ok(value) = secrets.get(key) {
                let usable_key = str::replace(key, ":", "_");
                secret_count += 1;
                secrets_as_env_vars.insert(usable_key, value.to_string());
            }
        }

        println!("ss2env is running jetp with {} secrets passed to jetp's environment.", secret_count);

        if let Ok(mut command) = Command::new(&config.target_command_args[0])
            .args(&config.target_command_args[1..])
            .envs(secrets_as_env_vars)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn() {
                match command.wait()  {
                    Ok(status) => {
                        if let Some(exit_code) = status.code() {
                            std::process::exit(exit_code);
                        }
                    }
                    Err(error) => {
                        println!("ss2env error encountered while running jetp, error was {:?}", error);
                        std::process::exit(4)
                    }
                }
        } else {
            println!("{} did not start.  Is it executable? ", &config.target_command_args[0]);
            std::process::exit(3)
        }

    } else {
        println!("Failed to load SecureStore vault.  Most likely the key file or store file cannot be parsed.  Check command line or use ssclient to test you have valid secrets file and secrets key. ");
        std::process::exit(2);
    }
}
