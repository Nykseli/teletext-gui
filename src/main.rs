mod html_parser;

fn main() {
    let file = "100.htm";
    let pobj = html_parser::HtmlLoader::new(&file);

    let mut parser = html_parser::TeleText::new();
    parser.parse(&pobj).unwrap();

    println!("  {}", parser.title);

    for row in parser.middle_rows {
        print!("    ");
        for html_item in row {
            match html_item {
                html_parser::HtmlItem::Text(text) => {
                    print!("{}", text);
                }
                html_parser::HtmlItem::Link(link) => {
                    print!("{}", link.inner_text);
                }
            }
        }
        println!();
    }
    println!();
}
