    /// Every word bound through `in_body` satisfies the full-chain proof that
    /// `in_body` itself checks only inductively (one parent link per word).
    #[test]
    fn every_bound_body_word_satisfies_the_full_chain_proof() {
        fn walk(word: &OriginalPackSourceWord, image: &SourceImage, config: LexerConfig, depth: usize, seen: &mut usize, deepest: &mut usize) {
            assert!(word.matches_source(image, config), "{}", word.captured_text());
            *seen += 1;
            *deepest = (*deepest).max(depth);
            if !word.is_braced() {
                return;
            }
            let rows = original_pack_source_statements(
                word.captured_text(),
                word.local.statement.config.nested(),
            );
            for (row, statement) in rows.iter().enumerate() {
                for ordinal in 0..statement.row().argv.len() {
                    if let Some(child) = word.body_word(row, ordinal) {
                        walk(&child, image, config, depth + 1, seen, deepest);
                    }
                }
            }
        }
        for source in [
            "speclib demo 1 {\n  command a {\n    option -x {\n      hover {x}\n      arty {1 {2 {3}}}\n    }\n  }\n  command b {arty 1; arty 2}\n}\n",
            "speclib demo 1 {\r\n  command a {\r\n    option -x {\r\n      hover {x\r\ny}\r\n    }\r\n  }\r\n}\r\n",
        ] {
            let config =
                super::super::pack_source_lexer_config(SourceSegmentationPurpose::WholeInput);
            let image = SourceImage::document(source);
            let (mut seen, mut deepest) = (0, 0);
            for statement in original_pack_source_statements(source, config) {
                for ordinal in 0..statement.row().argv.len() {
                    if let Some(local) = statement.local_word(ordinal) {
                        walk(&local.in_document(), &image, config, 0, &mut seen, &mut deepest);
                    }
                }
            }
            assert!(deepest >= 3, "{source:?} reached depth {deepest}");
            assert!(seen > 10, "{source:?} bound {seen} words");
        }
    }

