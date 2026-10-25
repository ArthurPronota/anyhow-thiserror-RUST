use thiserror::Error ;

#[derive(Error, Debug)]
enum StoreError {
    #[error("io: {0}")] // форматирование сообщения об ошибке
    Io( // всё что от std::io::Error
        #[from]
        std::io::Error
    ),

    #[error("missing key: {0}")] // форматирование сообщения об ошибке
    Missing(String),
}

// загрузить контент файла в String
fn load(path: &str) ->Result<String, StoreError> {
    Ok(
        std::fs::read_to_string(path)?
    )
}

fn main() ->Result<(), anyhow::Error>{
    let path = "Cargo.toml" ;

    match load(path) {
        Ok(v) => {
            println!("{v}") ;
            Ok(())
        },
        Err(err) => Err(err.into())
    }
}
