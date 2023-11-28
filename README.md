# ss2env
A small utility to pass secrets from SecureStore https://github.com/neosmart/securestore-rs archives to jetporch https://lists.sr.ht/~mpdehaan/jetporch-devel via environment variables.

# Important: Read this before you start

I want to be clear, if you have come here looking for the last word in secret management for your devops adventure,
the goal of this utility is not to provide bulletproof secrets, but to meet a simpler goal: _to keep sensitive strings out of 
your source code repository_.  Git is no place for secrets and nobody would ever choose to spend time scrubbing secrets from git.

Its up to you to protect secrets by not sharing logins, locking down file permissions so secrets and keys aren't visible
to other users and generally expecting the bad people to be good at taking advantage of any laziness, stale passwords
and other examples of not bringing your A game.

## Setup

Neosmart's SecureStore stores secrets in a json file which is intentionally human-readable, to allow secrets to be 
versioned alongside other code in your source code repository.

SecureStore provides a command line tool, `ssclient`, which can be used to generate secret files and manage
the secrets they hold.

Before deciding if you wish to use SecureStore secrets with jet
I recommended you install ssclient and create a secrets file
using the SecureStore walkthrough here:

https://crates.io/crates/ssclient

Welcome back!  To use ss2env, you'll need to tell it where the secret key and secret store files you need to use
are held.

Likely the easiest way is to set the two following environment variables
`
SS2ENV_KEY=/path/to/securestore/secrets.key
SS2ENV_STORE=/path/to/securestore/secrets.json
`
You can set these in the following file: `~/.ss2env`

This is convenient as you can then simply run `ss2env jetp <args to jet go here>`

If you don't want to use environment variables you can pass -s/--store and -k/--key to provide the paths to the store and key files respectively.

## What do I need to know?

The wrapper simply decrypts all the secrets in the secrets.json file
using the secrets.key and transforms them into environment variables
which are then passed to the jetp program.

### Transformations

#### Colons (:) become underscores (_)
Note that the SecureStore examples use colon (:) in the secret names,
but environment variable names containing : can cause problems, so any
colon (:) is converted to an underscore (_) as part of the conversion.

#### Jetp prefixing

Jetp prefixes all environment variables with `ENV_` so you
will need to prefix converted names in order to use secrets in templates.

See the following table for an example of how the names differ:

| *Original secret name* | *Environment variable passed to jet* | *Correct name to use in a jet template* |
|------------------------|--------------------------------------|-----------------------------------------|
| db:postgres_user       | db_postgres_user                     | ENV_db_postgres_user                    |

#### Environment is only passed to `jetp`

To make exfiltration slightly less easy, the program that ss2env passes to must be called `jetp`

#### ss2env is not aware of jetp arguments.

If you run jetp local -i inventory, jetp will error out (as of tech preview 1).
Because ss2env is unaware what arguments are passed to jetp, secrets will still be passed to 'jetp local' and in fact
any mode that jetp supports (provided the key and store are found and can be decrypted).

## Walkthrough

Here's a worked example

Install rust

see https://www.rust-lang.org/tools/install for instructions.

Install ssclient:

`cargo install ssclient` (or `cargo binstall ssclient` if you have added `binstall` to cargo).

Create a secrets.json file and export a secrets.key so secrets can be decrypted without a password:

```bash
cd ~
ssclient create secrets.json --export-key secrets.key
```

Add some secrets:

```bash
ssclient set db:username pgsql
ssclient set db:password
```

Fetch ss2env from github:

```bash
cd ~
git clone https://github.com/jhawkesworth/ss2env.git
````

Compile ss2env program:
```bash
cd ss2env
make
```

Copy the 'ss2env' executable to your path

```bash
cp target/release/ss2env somewhere/on/your/path
```

Create the `~/.ss2env` file so ss2env knows where to find the key and secret files:

```bash
echo > ~/.ss2env << EOF
SS2ENV_KEY=~/secrets.json
SS2ENV_STORE=~/secrets.key
EOF
```

Create a playbook to demonstrate secrets are passed to jet
```bash
mkdir ~/playbooks
vi ~/playbooks/template-test.yml
```
(enter the following then press `Esc :wq!`)
```yaml
- name: demonstrate templating

  groups:
    - all

  tasks:

    - !template
      src: /home/your_user/playbooks/secret_demo.txt
      dest: /tmp/secret_demo.txt

#  (note change /home/your_user to your user's home directory)
```
Create a template
```bash
vi ~/playbooks/secret_demo.txt
```

(enter the following then press `Esc :wq!`)
```yaml

# this is here to demonstrate that you can
# pass secrets through from securestore
# to jet templates

username: {{ENV_db_username}}
password: {{ENV_db_password}}

# end of template
```

Run the playbook via the wrapper
```bash
ss2env /path/to/jetp local -p ~/playbooks/template-test.yml
```

Check the templated values have been passed from securestore to jetp and stored 
in the templated output file.

```bash
cat /tmp/secret_demo.txt
```
The output should look like this:
```bash
# this is here to demonstrate that you can
# pass secrets through from securestore
# to jet templates

username: postgres
password: postgres1234
```
Clean up
```bash
rm /tmp/secret_demo.txt
```

# Suggested file organisation

One way to organise secrets is to have one login per environment that you are managing with jet.
This lets you keep your `secrets.key` in a hidden folder in your user's home directory, for example:
```bash
~/.ss2env/secrets.key
```
You can then store your `secrets.json` file in the same folder as your inventory files or dynamic 
inventory script.

So if your dynamic inventory is in `~/infrastructure/inventory/test/aws_ec2.py` you could have 
`secrets.json` in `~/infrastructure/inventory/test/secrets.json` by configuring as follows:
```bash
echo > ~/.ss2env << EOF
SS2ENV_KEY=~/.ss2env/secrets.key
SS2ENV_STORE=~/infrastructure/inventory/test/secrets.json
EOF
```

