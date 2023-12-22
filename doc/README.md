
## principle
* you should always maintain both `db_mysql.sql` and `db_sqlite.sql` at the same time.
* write unit tests always.

## auto generate new model
if you defined a new table ddl in [db_sqlite.sql](db_sqlite.sql) file,
and `cargo build --features=debug` will generate a new {table}.rs file under
`src/tables`, if u want to re-generate it , just delete the file and build again,
but always remember to use `git` to control the versions.