#include "tcl_lsp/lsp/folding_ranges.hpp"

#include "tcl_lsp/analysis/semantic_types.hpp"

#include <cstdint>
#include <set>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace tcl_lsp {
namespace {

using SeenRanges = std::set<std::pair<std::int32_t, std::int32_t>>;

void collect_scope_folds(const Scope& scope, std::vector<FoldingRangeInfo>& out,
                          SeenRanges& seen) {
    // Procs with multi-line bodies.
    for (auto& [name, proc] : scope.procs) {
        auto& br = proc.body_range;
        if (br.end.line > br.start.line) {
            auto key = std::make_pair(br.start.line, br.end.line);
            if (seen.insert(key).second) {
                // Use the name line as start for better folding UX.
                auto start = proc.name_range.start.line;
                if (start < br.end.line) {
                    out.push_back({start, br.end.line, FoldingRangeKind::REGION});
                }
            }
        }
    }

    // Namespace scopes with body ranges.
    for (auto& child : scope.children) {
        if (child->kind == ScopeKind::NAMESPACE && child->body_range.has_value()) {
            auto& br = *child->body_range;
            if (br.end.line > br.start.line) {
                auto key = std::make_pair(br.start.line, br.end.line);
                if (seen.insert(key).second) {
                    out.push_back({br.start.line, br.end.line, FoldingRangeKind::REGION});
                }
            }
        }
        collect_scope_folds(*child, out, seen);
    }
}

void collect_comment_folds(std::string_view source, std::vector<FoldingRangeInfo>& out,
                            SeenRanges& seen) {
    // Find consecutive comment lines (3+ lines → foldable).
    std::int32_t line = 0;
    std::int32_t comment_start = -1;
    std::int32_t comment_count = 0;

    auto flush_comment_block = [&]() {
        if (comment_count >= 3) {
            auto end_line = comment_start + comment_count - 1;
            auto key = std::make_pair(comment_start, end_line);
            if (seen.insert(key).second) {
                out.push_back({comment_start, end_line, FoldingRangeKind::COMMENT});
            }
        }
        comment_start = -1;
        comment_count = 0;
    };

    std::size_t pos = 0;
    while (pos <= source.size()) {
        // Find line content.
        auto nl = source.find('\n', pos);
        auto line_end = (nl == std::string_view::npos) ? source.size() : nl;
        auto line_text = source.substr(pos, line_end - pos);

        // Trim leading whitespace.
        auto trimmed = line_text;
        while (!trimmed.empty() && (trimmed[0] == ' ' || trimmed[0] == '\t'))
            trimmed.remove_prefix(1);

        bool is_comment = !trimmed.empty() && trimmed[0] == '#';

        if (is_comment) {
            if (comment_start < 0) {
                comment_start = line;
                comment_count = 1;
            } else {
                comment_count += 1;
            }
        } else {
            flush_comment_block();
        }

        if (nl == std::string_view::npos) break;
        pos = nl + 1;
        line += 1;
    }
    flush_comment_block();
}

} // anonymous namespace

auto get_folding_ranges(std::string_view source, const AnalysisResult* analysis)
    -> std::vector<FoldingRangeInfo> {
    std::vector<FoldingRangeInfo> ranges;
    SeenRanges seen;

    if (analysis != nullptr) {
        collect_scope_folds(analysis->global_scope(), ranges, seen);
    }

    collect_comment_folds(source, ranges, seen);

    return ranges;
}

} // namespace tcl_lsp
