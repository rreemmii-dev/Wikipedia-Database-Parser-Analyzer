mod lib {
    pub mod database {
        pub mod generate_database;
        pub mod load_database;
    }
    pub mod graph_utils {
        pub mod hashmap_graph_utils;
        pub mod vec_graph_utils;
    }
}

pub use crate::lib::*;
