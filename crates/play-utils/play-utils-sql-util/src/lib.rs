use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

fn inner_is_query_sql(sql: &str)->anyhow::Result<bool>{
    let sql = sql.trim();
    //parse sql
    let dialect = GenericDialect {}; // or AnsiDialect
    let statements = Parser::parse_sql(&dialect, sql)?;
    if statements.len()!=1{
        return Ok(false);
    }

    let is_query = match statements[0] {
        Statement::Query(_) => true,
        _ => false,
    };

    Ok(is_query)
}
pub fn is_query_sql(sql: &str)->bool{
    inner_is_query_sql(sql).unwrap_or_default()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(is_query_sql("select * from app"), true);
    }
}
