use serde::Serialize;
use sqlparser::ast::{ColumnOption, Statement, TableConstraint};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

#[derive(Debug, Serialize)]
pub struct TableColumn {
    pub name: String,
    pub ty: String,
}

#[derive(Debug,Serialize)]
pub struct TableInfo {
    pub table_name: String,
    pub pk_column: Option<String>,
    pub columns: Vec<TableColumn>,
}

pub fn parse_create_sql(sql: &str) -> Vec<TableInfo> {
    let dialect = GenericDialect {}; // or AnsiDialect
    let mut table_list = vec![];

    let ast = Parser::parse_sql(&dialect, sql).unwrap();
    for statement in ast {
        let mut table_name = "".to_string();
        let mut pk_column = None;
        let mut columns_list = vec![];

        match statement {
            Statement::CreateTable { name, columns, constraints, .. } => {
                table_name = name.to_string();
                // println!("table constraints : {:?}", constraints);


                for c in constraints {
                    match c {
                        TableConstraint::Unique { columns, is_primary, .. } => {
                            if is_primary {
                                pk_column = Some(columns[0].to_string());
                            }
                        }
                        _ => {}
                    }
                }


                for column in columns {
                    let col_name = column.name.to_string();
                    let col_type = column.data_type.to_string();
                    if pk_column.is_none() {
                        let is_pk = column.options.iter().filter(|c| {
                            match c.option {
                                ColumnOption::Unique { is_primary } => is_primary,
                                _ => false,
                            }
                        }).count() == 1;

                        if is_pk {
                            pk_column = Some(col_name.to_string());
                        }
                    }


                    // println!("col_name : {}, col_type; {}", col_name, col_type);
                    columns_list.push(TableColumn {
                        name: col_name.to_string(),
                        ty: col_type.to_string(),
                    });
                }

                table_list.push(TableInfo {
                    table_name,
                    pk_column,
                    columns: columns_list,
                });
            }
            _ => {}
        }



    }
    table_list
}


#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    use super::*;

    #[test]
    fn test_parse() -> anyhow::Result<()> {
        let sql = r#"


create table IF NOT EXISTS todo_item(
  id integer primary key AUTOINCREMENT,
  title varchar(255) not null,
  status varchar(10) not null
);
        "#;

        println!("{:?}", parse_create_sql(sql));

        Ok(())
    }
}