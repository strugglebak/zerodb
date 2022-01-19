pub mod query;

use sqlparser::parser::{Parser, ParserError};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::ast::Statement;

use crate::error::{Result, NollaDBError};
use crate::database::Database;
use crate::table::{Table, };

use query::create::{CreateQuery};
use query::insert::{InsertQuery};

#[derive(Debug, PartialEq)]
pub enum SQLQuery {
  CreateTable(String),
  Select(String),
  Insert(String),
  Update(String),
  Delete(String),
  Unknown(String),
}

impl SQLQuery {
  pub fn new(command: String) -> SQLQuery {
    let args: Vec<&str> = command.split_whitespace().collect();
    let first_cmd = args[0].to_owned();
    match first_cmd.as_ref() {
      "create" => SQLQuery::CreateTable(command),
      "select" => SQLQuery::Select(command),
      "insert" => SQLQuery::Insert(command),
      "update" => SQLQuery::Update(command),
      "delete" => SQLQuery::Delete(command),
      _ => SQLQuery::Unknown(command),
    }
  }
}

pub fn handle_sql_query(sql_query: &str, database: &mut Database) -> Result<String> {
  let dialect = SQLiteDialect {};
  let mut ast =
    Parser::parse_sql(&dialect, &sql_query)
      .map_err(NollaDBError::from)?;

  // 目前仅支持单个 SQL 语句输入
  if ast.len() > 1 {
    return Err(
      NollaDBError::SQLParseError(
        ParserError::ParserError(
          format!(
            "Expected a single SQL query statement
            , but here are '{}' SQL query statements,
            we now only support one single SQL query in typing",
            ast.len()
          )
        )
      )
    );
  }

  let message: String;
  let statement = ast.pop().unwrap();
  match statement {
    Statement::CreateTable {
      ..
    } => {
      match CreateQuery::new(&statement) {
        Ok(create_query) => {
          let CreateQuery {
            table_name,
            ..
          } = create_query;

          // 检查表是否已经被创建
          if database.has_table(table_name) {
            return Err(NollaDBError::Internal(
              format!(
                "Can not create table, because table '{}' already exists",
                table_name
              )
            ));
          }

          // 创建表
          let table = Table::new(create_query);
          // 把表插入到数据库中
          database.tables.insert(table_name.to_string(), table);
          // 打印表 schema
          table.print_column_of_schema();

          message = String::from("CREATE TABLE statement done");
        },
        Err(error) => return Err(error),
      }
    },
    Statement::Query(_) => {
      // TODO: 在表中查询
      message = String::from("SELECT statement done");
    },
    Statement::Insert {
      ..
    } => {
      match InsertQuery::new(&statement) {
        Ok(insert_query) => {
          let InsertQuery {
            table_name,
            table_column_names,
            table_column_values,
          } = insert_query;

          // 检查表是否已经被创建
          if !database.has_table(table_name) {
            return Err(NollaDBError::Internal(
              format!(
                "Table '{}' does not exist",
                table_name
              )
            ));
          }

          // 在对应表中执行插入操作
          let table = database.get_table_mut(table_name.to_string()).unwrap();
          // 检查要插入的 column name 是否在表中存在
          if !table_column_names
            .iter()
            .all(|column_name| table.has_column(column_name.to_string())) {
            return Err(NollaDBError::Internal(format!(
              "Can not insert, because some of the columns do not exist"
            )));
          }

          for table_column_value in table_column_values {
            // 1. 检查要插入的 column value 的个数是否和 column name 一致
            let v_len = table_column_value.len();
            let n_len = table_column_names.len();
            if v_len != n_len {
              return Err(NollaDBError::Internal(
                format!(
                  "{} values for {} columns",
                  v_len,
                  n_len
                )
              ));
            }

            // 2. 检查唯一约束
            if let Err(error) =
              table.check_unique_constraint(&table_column_names, &table_column_value) {
              return Err(NollaDBError::Internal(
                format!(
                  "Unique key constraint violation: {}",
                  error
                )
              ));
            }

            // 3. 以上 2 点检查完毕，说明没有唯一约束，可以插入
            table.insert_row(&table_column_names, &table_column_value);
          }

          // 打印插入完成后的表数据
          table.print_table_data();

          message = String::from("INSERT statement done");
        },
        Err(error) => return Err(error),
      }
    },
    Statement::Update {
      ..
    } => {
      // TODO: 在表中更新
      message = String::from("UPDATE statement done");
    },
    Statement::Delete {
      ..
    } => {
      // TODO: 在表中删除
      message = String::from("UPDATE statement done");
    },
    _ => {
      return Err(
        NollaDBError::ToBeImplemented(
          "Other SQL statement will to be implemented soon".to_string()
        )
      );
    },
  };

  println!("{}", message.to_string());
  Ok(message)
}
