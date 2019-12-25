extern crate chrono;
extern crate dotenv;
extern crate postgres;

use dotenv::dotenv;
use std::env;

use postgres::{Connection, TlsMode};

fn convert_type(t: &str) -> &str {
    match t {
        "timestamp without time zone" => "datetime",
        "text" => "nvarchar(100000)",
        "integer" => "INTEGER",
        "boolean" => "BOOLEAN",
        "bytea" => "BLOB",
        _ => "XXXXX",
    }
    /*
      if t == "timestamp without time zone" {
        "datetime"
      } else {
        "XXX unknown type XXX"
      }
    */
}

fn get_row_desc(row: &postgres::rows::Row, pk_field_name: &str) -> String {
    // let tbl_name: String = row.get("table_name");
    let col_name: String = row.get("column_name");
    // let col_pos: i32 = row.get("ordinal_position");
    let is_n: String = row.get("is_nullable");
    let is_nullable: bool = is_n == "YES";
    let data_type: String = row.get("data_type");
    let column_default: Option<String> = row.get("column_default");
    // println!("    -- tbl: {} col: {} pos: {} is_null: {} data_type {}", tbl_name, col_name, col_pos, is_nullable, data_type);
    // println!("    --  column_default: {:?}", column_default);

    // let def_val = "";

    let (primkey, def_val) = match column_default {
        None => (
            if &col_name == pk_field_name {
                " PRIMARY KEY"
            } else {
                ""
            }
            .to_string()
            .to_string(),
            "".to_string(),
        ),
        Some(dv) => {
            if dv.find("nextval").is_some() {
                (" PRIMARY KEY AUTOINCREMENT".to_string(), "".to_string())
            } else {
                let ret_pk = if &col_name == pk_field_name {
                    " PRIMARY KEY"
                } else {
                    ""
                }
                .to_string();
                let ret_dv = format!(
                    " DEFAULT {}",
                    if data_type == "boolean" {
                        if dv == "true" {
                            format!("1")
                        } else {
                            format!("0")
                        }
                    } else {
                        dv
                    }
                );
                (ret_pk, ret_dv)
            }
        }
    };

    let sqlite3_type = format!(
        "{} {}{}{}",
        convert_type(&data_type),
        if is_nullable { "NULL" } else { "NOT NULL" },
        primkey,
        def_val
    );

    format!("{} {}", col_name, sqlite3_type)
}

fn dump_table(conn: &postgres::Connection, curr_table_name: String) {
    // use postgres::types::Type;
    // use postgres::types::FromSql;

    let query_pk = "SELECT tc.constraint_name, tc.table_name, kcu.column_name, ccu.table_name AS foreign_table_name,
                  ccu.column_name AS foreign_column_name FROM 
      information_schema.table_constraints AS tc 
      JOIN information_schema.key_column_usage AS kcu
        ON tc.constraint_name = kcu.constraint_name
      JOIN information_schema.constraint_column_usage AS ccu
        ON ccu.constraint_name = tc.constraint_name
      WHERE constraint_type = 'PRIMARY KEY' AND tc.table_name=$1;";

    let mut pk_field_name: String = format!("");

    for row in &conn.query(query_pk, &[&curr_table_name]).unwrap() {
        pk_field_name = row.get("column_name");
    }
    println!("CREATE TABLE {}(", curr_table_name);
    let mut seen_field: bool = false;

    for row in &conn
        .query(
            "SELECT * FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2",
            &[&"public", &curr_table_name],
        )
        .unwrap()
    {
        if seen_field {
            print!(",\n");
        }
        print!("{}", get_row_desc(&row, &pk_field_name));
        seen_field = true;
    }

    let query2 = "SELECT tc.constraint_name, tc.table_name, kcu.column_name, ccu.table_name AS foreign_table_name,
                  ccu.column_name AS foreign_column_name FROM 
      information_schema.table_constraints AS tc 
      JOIN information_schema.key_column_usage AS kcu
        ON tc.constraint_name = kcu.constraint_name
      JOIN information_schema.constraint_column_usage AS ccu
        ON ccu.constraint_name = tc.constraint_name
      WHERE constraint_type = 'FOREIGN KEY' AND tc.table_name=$1;";

    for row in &conn.query(query2, &[&curr_table_name]).unwrap() {
        // let constraint_name: String = row.get("constraint_name");
        // let table_name: String = row.get("table_name");
        let column_name: String = row.get("column_name");
        let foreign_table_name: String = row.get("foreign_table_name");
        let foreign_column_name: String = row.get("foreign_column_name");
        // println!("{}   {}.{} => {}.{}", constraint_name, table_name, column_name, foreign_table_name, foreign_column_name);
        if seen_field {
            print!(",\n");
        }
        print!(
            "FOREIGN KEY({}) REFERENCES {} ({})",
            column_name, foreign_table_name, foreign_column_name
        );
        seen_field = true;
    }
    print!("\n);\n");
    let val_query = format!("SELECT * FROM \"{}\"", curr_table_name);
    for row in &conn.query(&val_query, &[]).unwrap() {
        let mut seen_val: bool = false;
        print!("INSERT INTO \"{}\" VALUES(", curr_table_name);
        for i in 0..row.len() {
            if seen_val {
                print!(",");
            };
            let c = row.columns();
            // print!("{:?}", c[i]);

            // println!("column {} type {:?}", i, c[i].type_());
            // let s_val: String = "unknown".to_string();
            let type_name = c[i].type_().name();
            let s_val: String = match type_name {
                "text" => {
                    let v: Option<String> = row.get(i);
                    match v {
                        None => "NULL".to_string(),
                        Some(iv) => format!("'{}'", iv.replace("'", "''")).to_string(),
                    }
                }
                "bool" => {
                    let v: Option<bool> = row.get(i);
                    match v {
                        None => "NULL".to_string(),
                        Some(iv) => format!("{}", if iv { 1 } else { 0 }).to_string(),
                    }
                }
                "int4" => {
                    let v: Option<i32> = row.get(i);
                    match v {
                        None => "NULL".to_string(),
                        Some(iv) => format!("{}", iv).to_string(),
                    }
                }
                "timestamp" => {
                    let v: Option<chrono::NaiveDateTime> = row.get(i);
                    match v {
                        None => "NULL".to_string(),
                        Some(iv) => format!("'{}'", iv).to_string(),
                    }
                }
                _ => format!("XXX-unknown-type-{}-XXX", type_name).to_string(),
            };

            print!("{}", s_val);
            seen_val = true;
        }
        println!(");");
        // println!("{:?}", row);
    }
}

/*

sqlite> DELETE FROM sqlite_sequence;
sqlite> INSERT INTO "sqlite_sequence" VALUES('Owners',3);
sqlite> INSERT INTO "sqlite_sequence" VALUES('SwitchTypes',1018);
sqlite> INSERT INTO "sqlite_sequence" VALUES('ModuleTypes',1029);
sqlite> INSERT INTO "sqlite_sequence" VALUES('Assets',2128);
*/

fn main() {
    dotenv().ok();

    println!("BEGIN TRANSACTION;");

    let url = env::var("POSTGRES_DB_URL").expect("POSTGRES_DB_URL must be set");

    let conn = Connection::connect(url, TlsMode::None).unwrap();

    for tbl_row in &conn.query("select table_name from information_schema.tables where table_schema='public' order by table_name", &[]).unwrap() {
      let curr_table_name: String = tbl_row.get("table_name");
      if curr_table_name.find("__diesel_schema").is_none() {
        dump_table(&conn, curr_table_name);
      }
   }
    println!("COMMIT;");

    /*
        let results = information_schema.columns // .filter(published.eq(true))
            .limit(1000)
            .load::<InfoSchema>(db.conn())
            .expect("Error loading assets");

        for a in results {
            println!("{:?}", a);
        }
    */
}
