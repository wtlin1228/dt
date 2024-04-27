use umya_spreadsheet::*;

use super::dependency_tracker::ModuleSymbol;

pub fn write_to_spreadsheet(filename: &str, paths: &Vec<Vec<ModuleSymbol>>) {
    let mut book = new_file();
    for (row_index, path) in paths.iter().enumerate() {
        for (column_index, module_symbol) in path.iter().enumerate() {
            let coordinate = (column_index as u32 + 1, row_index as u32 + 1);
            book.get_sheet_by_name_mut("Sheet1")
                .unwrap()
                .get_cell_mut(coordinate)
                .set_value(format!(
                    "{} ({})",
                    module_symbol.1.to_string(),
                    module_symbol.0
                ));
        }
    }
    let path = std::path::Path::new(filename);
    let _ = writer::xlsx::write(&book, path);
}
