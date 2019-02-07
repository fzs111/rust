use hir::db::HirDatabase;
use ra_syntax::{
    ast::{self, AstNode},
    TextUnit,
    SyntaxKind::{
        L_PAREN, R_PAREN, L_CURLY, R_CURLY, L_BRACK, R_BRACK, EXCL
    },
};
use crate::{AssistCtx, Assist};

pub(crate) fn remove_dbg(ctx: AssistCtx<impl HirDatabase>) -> Option<Assist> {
    let macro_call = ctx.node_at_offset::<ast::MacroCall>()?;

    if !is_valid_macrocall(macro_call, "dbg")? {
        return None;
    }

    let macro_range = macro_call.syntax().range();

    // If the cursor is inside the macrocall, we'll try to maintain
    // the cursor position by subtracting the length of dbg!( from the start
    // of the filerange, otherwise we'll default to using the start of the macrocall
    let cursor_pos = {
        let file_range = ctx.frange.range;

        let offset_start = file_range
            .start()
            .checked_sub(macro_range.start())
            .unwrap_or_else(|| TextUnit::from(0));

        let dbg_size = TextUnit::of_str("dbg!(");

        if offset_start > dbg_size {
            file_range.start() - dbg_size
        } else {
            macro_range.start()
        }
    };

    let macro_content = {
        let macro_args = macro_call.token_tree()?.syntax();
        let range = macro_args.range();
        let start = range.start() + TextUnit::of_char('(');
        let end = range.end() - TextUnit::of_char(')');

        macro_args.text().slice(start..end).to_string()
    };

    ctx.build("remove dbg!()", |edit| {
        edit.replace(macro_range, macro_content);
        edit.set_cursor(cursor_pos);
    })
}

/// Verifies that the given macro_call actually matches the given name
/// and contains proper ending tokens
fn is_valid_macrocall(macro_call: &ast::MacroCall, macro_name: &str) -> Option<bool> {
    let path = macro_call.path()?;
    let name_ref = path.segment()?.name_ref()?;

    // Make sure it is actually a dbg-macrocall, dbg followed by !
    let excl = path.syntax().next_sibling()?;

    if name_ref.text() != macro_name || excl.kind() != EXCL {
        return None;
    }

    let node = macro_call.token_tree()?.syntax();
    let first_child = node.first_child()?;
    let last_child = node.last_child()?;

    match (first_child.kind(), last_child.kind()) {
        (L_PAREN, R_PAREN) | (L_BRACK, R_BRACK) | (L_CURLY, R_CURLY) => Some(true),
        _ => Some(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::{check_assist, check_assist_not_applicable};

    #[test]
    fn test_remove_dbg() {
        check_assist(remove_dbg, "<|>dbg!(1 + 1)", "<|>1 + 1");

        check_assist(remove_dbg, "dbg!<|>((1 + 1))", "<|>(1 + 1)");

        check_assist(remove_dbg, "dbg!(1 <|>+ 1)", "1 <|>+ 1");

        check_assist(remove_dbg, "let _ = <|>dbg!(1 + 1)", "let _ = <|>1 + 1");

        check_assist(
            remove_dbg,
            "
fn foo(n: usize) {
    if let Some(_) = dbg!(n.<|>checked_sub(4)) {
        // ...
    }
}
",
            "
fn foo(n: usize) {
    if let Some(_) = n.<|>checked_sub(4) {
        // ...
    }
}
",
        );
    }
    #[test]
    fn test_remove_dbg_with_brackets_and_braces() {
        check_assist(remove_dbg, "dbg![<|>1 + 1]", "<|>1 + 1");
        check_assist(remove_dbg, "dbg!{<|>1 + 1}", "<|>1 + 1");
    }

    #[test]
    fn test_remove_dbg_not_applicable() {
        check_assist_not_applicable(remove_dbg, "<|>vec![1, 2, 3]");
        check_assist_not_applicable(remove_dbg, "<|>dbg(5, 6, 7)");
        check_assist_not_applicable(remove_dbg, "<|>dbg!(5, 6, 7");
    }
}
