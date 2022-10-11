extern crate dotenv;
extern crate rusqlite;
use dotenv::dotenv;
use std::env;

use rusqlite::Connection;
use rusqlite::Result;
extern crate chrono;
// use rusqlite::types::{FromSql, FromSqlResult, ValueRef, ToSql, ToSqlOutput, Value, FromSqlError};
use rusqlite::types::Value;

// extern crate time;

fn convert_type(t: &str) -> String {
    let unknown = format!("XXXXX[{}]", t);
    let out = if t == "datetime" {
        "timestamp without time zone"
    } else if t.to_lowercase() == "timestamp" {
        "timestamp without time zone"
    } else if t.to_lowercase().starts_with("char(") {
        "text"
    } else if t.to_lowercase().starts_with("nchar(") {
        "text"
    } else if t.to_lowercase().starts_with("varchar(") {
        "text"
    } else if t.to_lowercase().starts_with("nvarchar(") {
        "text"
    } else if t.to_lowercase() == "varchar" {
        "text"
    } else if t == "INTEGER" {
        "integer"
    } else if t == "INT" {
        "integer"
    } else if t == "int" {
        "integer"
    } else if t == "BOOLEAN" {
        "boolean"
    } else if t == "BLOB" {
        "bytea"
    } else {
        "XXXXX"
    };
    if out == "XXXXX" {
        unknown.to_string()
    } else {
        out.to_string()
    }
}

#[derive(Debug)]
struct ColInfo {
    num: i32,
    name: String,
    typ: String,
    dflt: Option<String>,
    not_nul: i32,
    key: i32,
}

fn convert_def_value(dv: &str, data_type: &str) -> String {
    match data_type {
        "boolean" => match dv {
            "0" => "false",
            "1" => "true",
            "false" => "false",
            "true" => "true",
            "NULL" => "NULL",
            _ => "NULL",
        }
        .to_string(),
        "text" => format!("'{}'", dv).to_string(),
        _ => dv.clone().to_string(),
    }
}

fn get_row_desc(row: &ColInfo) -> String {
    let col_name: String = row.name.clone();
    let is_nullable: bool = row.not_nul == 0; /* not not nul is nullable */
    let data_type: String = row.typ.clone();
    let postgres_type = convert_type(&data_type);
    let column_default: Option<String> = row.dflt.clone(); // None; // row.get_unwrap("column_default");
                                                           // println!("    -- tbl: {} col: {} pos: {} is_null: {} data_type {}", tbl_name, col_name, col_pos, is_nullable, data_type);
                                                           // println!("    --  column_default: {:?}", column_default);

    let (primkey, def_val) = match column_default {
        None => ("".to_string(), "".to_string()),
        Some(dv) => {
            if dv.find("nextval").is_some() {
                (" PRIMARY KEY AUTOINCREMENT".to_string(), "".to_string())
            } else {
                (
                    "".to_string(),
                    format!(" DEFAULT {}", convert_def_value(&dv, &postgres_type)),
                )
            }
        }
    };

    let sqlite3_type = format!(
        "{}{}{}{}",
        postgres_type,
        def_val,
        if is_nullable { "" } else { " NOT NULL" },
        primkey
    );

    format!("    \"{}\" {}", col_name, sqlite3_type)
}

fn for_each_table<F>(conn: &Connection, mut some_closure: F)
where
    F: FnMut(&Connection, &str),
{
    let mut stmt = conn
        .prepare("SELECT name FROM \"sqlite_master\" WHERE type='table' order by name;")
        .unwrap();

    let row_iter = stmt
        .query_map([], |row| {
            let tbl_name: String = row.get_unwrap(0);
            Ok(tbl_name)
        })
        .unwrap();
    for tblname in row_iter {
        let curr_table_name: String = tblname.unwrap();
        if curr_table_name.find("sqlite_sequence").is_none() {
            some_closure(&conn, &curr_table_name);
        }
    }
}

fn for_each_column<F>(conn: &Connection, curr_table_name: &str, mut some_closure: F)
where
    F: FnMut(&Connection, &str, &ColInfo),
{
    let pragma_sql = format!("PRAGMA table_info('{}');", curr_table_name);

    let mut stmt = conn.prepare(&pragma_sql).unwrap();

    let row_iter = stmt
        .query_map([], |row| {
            let col_info = ColInfo {
                num: row.get_unwrap(0),
                name: row.get_unwrap(1),
                typ: row.get_unwrap(2),
                not_nul: row.get_unwrap(3),
                dflt: row.get_unwrap(4),
                key: row.get_unwrap(5),
            };
            Ok(col_info)
        })
        .unwrap();

    for col in row_iter {
        let col1 = col.unwrap();
        some_closure(conn, curr_table_name, &col1);
    }
}

fn for_each_unique_column<F>(conn: &Connection, mut some_closure: F)
where
    F: FnMut(&Connection, &str, &str),
{
    let pragma_sql = format!(
        "SELECT DISTINCT m.name as table_name, ii.name as column_name
  FROM sqlite_master AS m,
       pragma_index_list(m.name) AS il,
       pragma_index_info(il.name) AS ii
 WHERE m.type='table' AND il.[unique] = 1;"
    );

    let mut stmt = conn.prepare(&pragma_sql).unwrap();

    let row_iter = stmt
        .query_map([], |row| {
            let tab: String = row.get_unwrap(0);
            let col: String = row.get_unwrap(1);
            Ok((tab, col))
        })
        .unwrap();

    for col in row_iter {
        let (table_name, column_name) = col.unwrap();
        some_closure(conn, &table_name, &column_name);
    }
}

/*
sqlite> pragma foreign_key_list("Addresses")
   ...> ;
0|0|Switches|SwitchID|SwitchID|NO ACTION|NO ACTION|NONE
1|0|Services|ServiceID|ServiceID|NO ACTION|NO ACTION|NONE
*/

#[derive(Debug)]
struct ForeignKeyInfo {
    num: i32,
    something: i32,
    fk_table: String,
    field: String,
    fk_field: String,
}

fn for_each_fk_column<F>(conn: &Connection, curr_table_name: &str, mut some_closure: F)
where
    F: FnMut(&Connection, &str, &ForeignKeyInfo),
{
    let pragma_sql = format!("PRAGMA foreign_key_list('{}');", curr_table_name);

    let mut stmt = conn.prepare(&pragma_sql).unwrap();

    let row_iter = stmt
        .query_map([], |row| {
            let col_info = ForeignKeyInfo {
                num: row.get_unwrap(0),
                something: row.get_unwrap(1),
                fk_table: row.get_unwrap(2),
                field: row.get_unwrap(3),
                fk_field: row.get_unwrap(4),
            };
            Ok(col_info)
        })
        .unwrap();

    for col in row_iter {
        let col1 = col.unwrap();
        some_closure(conn, curr_table_name, &col1);
    }
}

fn print_data_dump(
    conn: &Connection,
    curr_table_name: &str,
    prim_key: &str,
    all_types: &Vec<String>,
) {
    let order_by = if prim_key == "" {
        format!("")
    } else {
        format!(" ORDER BY \"{}\"", prim_key)
    };
    let pragma_sql = format!("SELECT * FROM \"{}\"{};", curr_table_name, order_by);

    let mut stmt = conn.prepare(&pragma_sql).unwrap();

    let num_columns = stmt.column_count();
    let mut rows = (stmt.query([])).unwrap();

    while let Ok(Some(row)) = rows.next() {
        let mut out: String = format!("INSERT INTO \"{}\" VALUES (", curr_table_name).to_string();
        let mut maybe_comma = "";

        for i in 0..num_columns {
            out.push_str(maybe_comma);
            maybe_comma = ",";
            // let thing = row.get_unwrap::<i32, bool>(i);
            if all_types[i as usize] == "boolean" {
                let maybe_bool: Result<bool> = row.get(i);
                match maybe_bool {
                    Ok(_b_val) => {
                        let b_val: bool = row.get_unwrap(i);
                        let col_val = format!("{}", b_val);
                        out.push_str(&col_val);
                    }
                    Err(_e) => {
                        let col_val = format!("NULL");
                        out.push_str(&col_val);
                    }
                }
            } else {
                let thing = row.get_unwrap::<_, Value>(i);
                let col_val = match thing {
                    Value::Null => format!("NULL"),
                    Value::Integer(iv) => format!("{}", iv),
                    Value::Real(rv) => format!("{}", rv),
                    Value::Text(sv) => format!("'{}'", sv.replace("'", "''")),
                    Value::Blob(bv) => format!("{:?}", bv),
                };
                out.push_str(&col_val);
            }
        }
        out.push_str(");");
        println!("{}", out);
    }
}

fn dump_table(pg_user: &str, conn: &rusqlite::Connection, curr_table_name: &str, prim_key: &str) {
    println!("--");
    println!(
        "-- Name: {}; Type: TABLE; Schema: public; Owner: {}",
        curr_table_name, pg_user
    );
    println!("--\n");
    println!("CREATE TABLE \"{}\" (", curr_table_name);
    let mut seen_field: bool = false;

    let mut all_columns: Vec<ColInfo> = vec![];
    let mut all_types: Vec<String> = vec![];

    for_each_column(conn, curr_table_name, |_conn, _tbl, col1| {
        if seen_field {
            print!(",\n");
        }
        // println!("{:?}", col1);
        print!("{}", get_row_desc(&col1));
        let info = ColInfo {
            num: col1.num,
            name: col1.name.clone(),
            typ: col1.typ.clone(),
            dflt: col1.dflt.clone(),
            not_nul: col1.not_nul,
            key: col1.key,
        };
        all_columns.push(info);
        all_types.push(convert_type(&col1.typ).to_string());
        seen_field = true;
    });
    println!("\n);\n");
    println!(
        "\nALTER TABLE \"{}\" OWNER TO {};\n",
        curr_table_name, pg_user
    );
    print_data_dump(conn, curr_table_name, prim_key, &all_types);
}

static POSTGRES_PREAMBLE: &'static str = include_str!("pg-preamble.txt");
static POSTGRES_POSTAMBLE: &'static str = include_str!("pg-postamble.txt");
static _POSTGRES_TWEAKS: &'static str = include_str!("pg-tweaks.txt");

fn seq_name(table: &str, column: &str) -> String {
    let seqname = format!("seq_{}_{}", table, column);
    seqname
}

fn idx_name(table: &str, column: &str) -> String {
    let seqname = format!("idx_{}_by_{}", table, column);
    seqname
}

fn print_sequence(pg_user: &str, table: &str, column: &str) {
    let seqname = seq_name(table, column);
    println!("--");
    println!(
        "-- Name: {}; Type: SEQUENCE; Schema: public; Owner: {}",
        seqname, pg_user
    );
    println!("--\n");
    println!("CREATE SEQUENCE \"{}\"", seqname);
    println!("    START WITH 1");
    println!("    INCREMENT BY 1");
    println!("    NO MINVALUE");
    println!("    NO MAXVALUE");
    println!("    CACHE 1;\n\n");
    println!("ALTER TABLE \"{}\" OWNER TO {};\n", seqname, pg_user);
    println!("--");
    println!(
        "-- Name: {}; Type: SEQUENCE OWNED BY; Schema: public; Owner: {}",
        seqname, pg_user
    );
    println!("--\n");
    println!(
        "ALTER SEQUENCE \"{}\" OWNED BY \"{}\".\"{}\";\n\n",
        seqname, table, column
    );
}

fn print_sequence_assign(pg_user: &str, table: &str, column: &str) {
    let seqname = seq_name(table, column);
    println!("--");
    println!(
        "-- Name: {}; Type: DEFAULT; Schema: public; Owner: {}",
        column, pg_user
    );
    println!("--\n");
    println!(
        "ALTER TABLE ONLY \"{}\" ALTER COLUMN \"{}\" SET DEFAULT nextval('\"{}\"'::regclass);",
        table, column, seqname
    );
    println!("\n");
}

fn print_constraint_pkey(pg_user: &str, table: &str, column: &str) {
    let idxname = idx_name(table, column);
    println!("--");
    println!(
        "-- Name: {}; Type: CONSTRAINT; Schema: public; Owner: {}",
        idxname, pg_user
    );
    println!("--\n");
    println!("ALTER TABLE ONLY \"{}\"", table);
    println!(
        "    ADD CONSTRAINT \"{}\" PRIMARY KEY (\"{}\");\n\n",
        idxname, column
    );
}

fn print_constraint_unique(pg_user: &str, table: &str, column: &str) {
    let idxname = format!("{}_unique", &idx_name(table, column));
    println!("--");
    println!(
        "-- Name: {}; Type: CONSTRAINT; Schema: public; Owner: {}",
        idxname, pg_user
    );
    println!("--\n");
    println!("ALTER TABLE ONLY \"{}\"", table);
    println!(
        "    ADD CONSTRAINT \"{}\" UNIQUE (\"{}\");\n\n",
        idxname, column
    );
}

fn fk_name(table: &str, column: &str) -> String {
    let fkname = format!("{}_{}_fkey", table, column);
    fkname
}

fn print_constraint_fkey(pg_user: &str, table: &str, col: &ForeignKeyInfo) {
    let fkname = fk_name(table, &col.field);
    println!("--");
    println!(
        "-- Name: {}; Type: FK CONSTRAINT; Schema: public; Owner: {}",
        fkname, pg_user
    );
    println!("--\n");
    println!("ALTER TABLE ONLY \"{}\"", table);
    println!(
        "    ADD CONSTRAINT \"{}\" FOREIGN KEY (\"{}\") REFERENCES \"{}\"(\"{}\");\n\n",
        fkname, col.field, col.fk_table, col.fk_field
    );
}

fn sql_from_scratch(pg_user: &str, conn: &Connection) {
    use std::collections::HashMap;
    let mut foreign_keys = HashMap::new();
    let mut primary_keys = HashMap::new();

    println!("{}", POSTGRES_PREAMBLE.replace("{{pg_user}}", pg_user));
    println!("BEGIN TRANSACTION;");

    for_each_table(&conn, |conn, t| {
        for_each_fk_column(conn, t, |_conn, t, col| {
            foreign_keys.insert(format!("{}.{}", t, col.field), "yes");
        });
        for_each_column(conn, t, |_conn, t, col| {
            if col.key == 1 {
                primary_keys.insert(format!("{}", t), col.name.clone());
            }
        });
    });

    for_each_table(&conn, |conn, t| {
        let pkey = primary_keys.get(t).unwrap();
        dump_table(pg_user, conn, t, &pkey);
    });

    // dump the sequence creation statements
    for_each_table(&conn, |conn, t| {
        for_each_column(conn, t, |_conn, t, col| {
            let fkk = format!("{}.{}", t, &col.name);
            let is_not_fk = foreign_keys.get(&fkk).is_none();
            if col.key == 1 && convert_type(&col.typ) == "integer" && is_not_fk {
                print_sequence(pg_user, t, &col.name);
            }
        });
    });
    // dump the sequence assign statements
    for_each_table(&conn, |conn, t| {
        for_each_column(conn, t, |_conn, t, col| {
            let fkk = format!("{}.{}", t, &col.name);
            let is_not_fk = foreign_keys.get(&fkk).is_none();
            if col.key == 1 && convert_type(&col.typ) == "integer" && is_not_fk {
                print_sequence_assign(pg_user, t, &col.name);
            }
        });
    });
    // dump the primary key statements
    for_each_table(&conn, |conn, t| {
        for_each_column(conn, t, |_conn, t, col| {
            if col.key == 1 {
                print_constraint_pkey(pg_user, t, &col.name);
            }
        });
    });
    // dump the unique constraints
    for_each_unique_column(&conn, |_conn, t, c| {
        print_constraint_unique(pg_user, t, c);
    });
    // dump the foreign key statements
    for_each_table(&conn, |conn, t| {
        for_each_fk_column(conn, t, |_conn, t, col| {
            print_constraint_fkey(pg_user, t, col);
        });
    });
    println!("{}", POSTGRES_POSTAMBLE);
    println!("COMMIT;");
}

fn main() {
    dotenv().ok();

    let url = env::var("SQLITE3_DB_URL").expect("SQLITE3_DB_URL must be set");
    let pg_user = env::var("POSTGRES_DB_USER").expect("POSTGRES_DB_USER must be set");

    let conn = Connection::open(url).unwrap();

    sql_from_scratch(&pg_user, &conn);
}
