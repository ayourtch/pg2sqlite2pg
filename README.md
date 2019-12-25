# pg2sqlite2pg
PostgreSQL db to sqlite3 and back (at least for some databases)

This is a small tool that I wrote to help me move back and forth
between sqlite3 and PostgreSQL database representations for another
project, and I realized it does not have any dependencies and can
be useful elsewhere. Thus, it is a little separate project now.

It probably is not usable as a general-purpose tool for a random database,
but might be useful for someone to hack on and adapt to their needs.

## Building

    cargo build

## Running

### Exporting PostgreSQL database

Exporting PostgreSQL will output SQL that should be ready to pipe to sqlite3:

    POSTGRES_DB_URL=postgresql://username:password@localhost/database ./target/debug/export-postgres-to-sqlite3


### Exporting Sqlite3 database

Exporting Sqlite3 database will output SQL that should be ready to pipe to "sudo -u postgres psql POSTGRESDBURL -f FILE":

    SQLITE3_DB_URL=/path/to/sqlite3.db POSTGRES_DB_USER=pguser ./target/debug/export-sqlite3-to-postgres


