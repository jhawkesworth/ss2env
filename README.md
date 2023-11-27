# ss2env
A small utility to pass secrets from SecureStore https://github.com/neosmart/securestore-rs archives to jetporch https://lists.sr.ht/~mpdehaan/jetporch-devel via environment variables.

## Important: Read this before you start

I want to be clear, if you have come here looking for the last word in secret management for your devops adventure,
the goal of this utility is not to provide bulletproof secrets, but to meet a simpler goal to keep sensitive strings out of 
your source code repository.

Its up to you to protect secrets by not sharing logins, locking down file permissions so secrets and keys aren't visible
to other users and generally expecting the bad people to be good at taking advantage of any laziness, stale passwords
and other examples of not bringing your A game.

## Setup

Neosmart's SecureStore stores secrets in a json file which is intentionally human-readable, to allow secrets to be 
versioned alongside other code in your source code repository.

SecureStore provides a command line tool, ssclient, which can be used to generate secret files and manage
the secrets they hold.

Before deciding if you wish to use SecureStore secrets with jet
it is recommended to install ssclient and create a secrets file
using the walk through here:

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

## How are secrets processed?

The wrapper simply decrypts all the secrets in the secrets.json file
using the secrets.key and transforms them into environment variables
which are then passed to the jetp program.

### Transformations

### Colon (:) become underscore (_)
Note that the SecureStore examples use colon (:) in the secret names,
but environment variable names containing : can cause problems, so any
colon (:) is converted to an underscore (_) as part of the conversion.

### Jetp prefixing

Jetp prefixes all environment variables with ENV_ so you
will need to prefix converted names in order to use secrets in templates.

See the following table for an example of how the names differ:

| original secret name | environment variable passed to jet | correct name to use in a jet template |
| db:postgres_user | db_postgres_user | ENV_db_postgres_user |

### environment only passed to `jetp`

To make exfiltration slightly less easy, the program that ss2env passes to must be called `jetp`

### ss2env is not aware of jetp arguments.

If you run jetp local -i inventory, jetp will error out (as of tech preview 2).
Because ss2env is unaware what arguments are passed to jetp, secrets will still be passed to 'jetp local' and in fact
any mode that jetp supports (provided the key and store are found and can be decrypted).

## Walkthrough

A worked example

#install rust
see https://www.rust-lang.org/tools/install

# install ssclient
cargo binstall ssclient

# create a secrets.json file and export a secrets.key so secrets can be decrypted without a password
cd ~
ssclient create secrets.json --export-key secrets.key

# add some secrets
ssclient set db:username pgsql
ssclient set db:password

# compile the wrapper program, ss2env, 
# fetch ss2env from github:
cd ~
git clone https://github.com/jhawkesworth/ss2env.git
cd ss2env
# compile the wrapper program
make

# you can now copy the 'ss2env' executable to your path

cp target/release/ss2env somewhere/on/your/path

# create the ~/.ss2env file so ss2env knows where to find the key and secret files
echo > ~/.ss2env << EOF
SS2ENV_KEY=~/secrets.json
SS2ENV_STORE=~/secrets.key
EOF

# create a playbook to demonstrate secrets are passed to jet
mkdir ~/playbooks

$ vi ~/playbooks/template-test.yml
(enter the following then press Esc :wq!)
- name: demonstrate templating

  groups:
    - all

  tasks:

    - !template
      src: /home/your_user/playbooks/secret_demo.txt
      dest: /tmp/secret_demo.txt

#  (note change /home/your_user to your user's home directory)

# create a template
# vi ~/playbooks/secret_demo.txt
(enter the following then press Esc :wq!)

# this is here to demo you can
# pass secrets through from securestore
# to jet templates

username: {{ENV_db_username}}
password: {{ENV_db_password}}

# end of template

# run the playbook via the wrapper
ss2env --store secrets.json --key secrets.key -- /path/to/jetp local -p ~/playbooks/template-test.yml

# check the templated values have passed through jetp to the templated file.

cat /tmp/secret_demo.txt
# this is here to demo you can
# pass secrets through from securestore
# to jet templates

username: postgres
password: postgres1234

# clean up

rm /tmp/secret_demo.txt

# Suggested file organisation

One way to organise secrets is to have one login per environment that you are managing with jet.
This lets you keep your secrets.key in a hidden folder in your user's home directory
~/.ss2env/secrets.key
You can then store your secrets.json file in the same folder as your inventory files or dynamic inventory script.
So if your dynamic inventory is in ~/infrastructure/inventory/test/aws_ec2.py you could have secrets.json in
~/infrastructure/inventory/test/secrets.json

echo > ~/.ss2env << EOF
SS2ENV_KEY=~/.ss2env/secrets.key
SS2ENV_STORE=~/infrastructure/inventory/test/secrets.json
EOF

# git is no place for secrets

Scrubbing secrets from git is not something anyone wants to spend time on.  
Take a beat to set things up so secrets stay out of git and sleep better at night.
