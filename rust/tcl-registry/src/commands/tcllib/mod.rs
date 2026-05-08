//! `tcllib` command specifications.

#![allow(non_snake_case)]

mod base64__decode;
mod base64__encode;
mod cmdline__getargv0;
mod cmdline__getfiles;
mod cmdline__getknownopt;
mod cmdline__getknownoptions;
mod cmdline__getopt;
mod cmdline__getoptions;
mod cmdline__typedgetopt;
mod cmdline__typedgetoptions;
mod cmdline__typedusage;
mod cmdline__usage;
mod csv__iscomplete;
mod csv__join;
mod csv__joinlist;
mod csv__joinmatrix;
mod csv__read2matrix;
mod csv__read2queue;
mod csv__report;
mod csv__split;
mod csv__split2matrix;
mod csv__split2queue;
mod csv__writematrix;
mod csv__writequeue;
mod dns__address;
mod dns__cleanup;
mod dns__cname;
mod dns__configure;
mod dns__dump;
mod dns__error;
mod dns__errorcode;
mod dns__name;
mod dns__reset;
mod dns__resolve;
mod dns__result;
mod dns__status;
mod dns__wait;
mod fileutil__appendtofile;
mod fileutil__cat;
mod fileutil__filetype;
mod fileutil__find;
mod fileutil__findbypattern;
mod fileutil__foreachline;
mod fileutil__fullnormalize;
mod fileutil__grep;
mod fileutil__insertintofile;
mod fileutil__install;
mod fileutil__jail;
mod fileutil__lexnormalize;
mod fileutil__maketempdir;
mod fileutil__relative;
mod fileutil__relativeurl;
mod fileutil__removefromfile;
mod fileutil__replaceinfile;
mod fileutil__stripn;
mod fileutil__strippwd;
mod fileutil__tempdir;
mod fileutil__tempdirreset;
mod fileutil__tempfile;
mod fileutil__test;
mod fileutil__touch;
mod fileutil__updateinplace;
mod fileutil__writefile;
mod html__html_entities;
mod html__tagstrip;
mod ip__collapse;
mod ip__contract;
mod ip__equal;
mod ip__is;
mod ip__mask;
mod ip__normalize;
mod ip__prefix;
mod ip__subtract;
mod ip__type;
mod ip__version;
mod json__dict2json;
mod json__json2dict;
mod json__list2json;
mod json__many_json2dict;
mod json__string2json;
mod json__validate;
mod logger__disable;
mod logger__enable;
mod logger__import;
mod logger__init;
mod logger__initnamespace;
mod logger__levels;
mod logger__servicecmd;
mod logger__services;
mod logger__setlevel;
mod logger__walk;
mod math__statistics__analyse_kruskal_wallis;
mod math__statistics__autocorr;
mod math__statistics__basic_stats;
mod math__statistics__control_rchart;
mod math__statistics__control_xbar;
mod math__statistics__corr;
mod math__statistics__crosscorr;
mod math__statistics__filter;
mod math__statistics__group_rank;
mod math__statistics__histogram;
mod math__statistics__histogram_alt;
mod math__statistics__interval_mean_stdev;
mod math__statistics__lillieforsfit;
mod math__statistics__linear_model;
mod math__statistics__linear_residuals;
mod math__statistics__map;
mod math__statistics__max;
mod math__statistics__mean;
mod math__statistics__mean_histogram_limits;
mod math__statistics__median;
mod math__statistics__min;
mod math__statistics__minmax_histogram_limits;
mod math__statistics__number;
mod math__statistics__print_2x2;
mod math__statistics__pstdev;
mod math__statistics__pvar;
mod math__statistics__quantiles;
mod math__statistics__samplescount;
mod math__statistics__spearman_rank;
mod math__statistics__spearman_rank_extended;
mod math__statistics__stdev;
mod math__statistics__t_test_mean;
mod math__statistics__test_2x2;
mod math__statistics__test_anova_f;
mod math__statistics__test_duckworth;
mod math__statistics__test_dunnett;
mod math__statistics__test_kruskal_wallis;
mod math__statistics__test_normal;
mod math__statistics__test_rchart;
mod math__statistics__test_tukey_range;
mod math__statistics__test_wilcoxon;
mod math__statistics__test_xbar;
mod math__statistics__var;
mod md5__md5;
mod mime__buildmessage;
mod mime__copymessage;
mod mime__field_decode;
mod mime__finalize;
mod mime__getbody;
mod mime__getcontenttype;
mod mime__getheader;
mod mime__getproperty;
mod mime__getsize;
mod mime__gettransferencoding;
mod mime__initialize;
mod mime__mapencoding;
mod mime__parseaddress;
mod mime__parsedatetime;
mod mime__reversemapencoding;
mod mime__setheader;
mod mime__uniqueid;
mod mime__word_decode;
mod mime__word_encode;
mod sha1__sha1;
mod sha2__sha256;
mod smtp__sendmessage;
mod snit__compile;
mod snit__macro;
mod snit__method;
mod snit__type;
mod snit__typemethod;
mod snit__widget;
mod snit__widgetadaptor;
mod struct__list;
mod struct__queue;
mod struct__set;
mod struct__stack;
mod textutil__adjust;
mod textutil__blank;
mod textutil__cap;
mod textutil__capeachword;
mod textutil__chop;
mod textutil__indent;
mod textutil__longestcommonprefix;
mod textutil__longestcommonprefixlist;
mod textutil__splitn;
mod textutil__splitx;
mod textutil__strrepeat;
mod textutil__tabify;
mod textutil__tabify2;
mod textutil__tail;
mod textutil__trim;
mod textutil__trimemptyheading;
mod textutil__trimleft;
mod textutil__trimprefix;
mod textutil__trimright;
mod textutil__uncap;
mod textutil__undent;
mod textutil__untabify;
mod textutil__untabify2;
mod uri__canonicalize;
mod uri__geturl;
mod uri__isrelative;
mod uri__join;
mod uri__register;
mod uri__resolve;
mod uri__setquirkoption;
mod uri__split;
mod uuid__uuid;
mod yaml__dict2yaml;
mod yaml__huddle2yaml;
mod yaml__list2yaml;
mod yaml__setoptions;
mod yaml__yaml2dict;
mod yaml__yaml2huddle;

use crate::spec::CommandSpec;

/// Return all `tcllib` command specifications.
// Flat declarative `vec![spec(), ...]` — splitting hurts
// readability for a one-shot table.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn tcllib_command_specs() -> Vec<CommandSpec> {
    vec![
        base64__decode::spec(),
        base64__encode::spec(),
        cmdline__getargv0::spec(),
        cmdline__getfiles::spec(),
        cmdline__getknownopt::spec(),
        cmdline__getknownoptions::spec(),
        cmdline__getopt::spec(),
        cmdline__getoptions::spec(),
        cmdline__typedgetopt::spec(),
        cmdline__typedgetoptions::spec(),
        cmdline__typedusage::spec(),
        cmdline__usage::spec(),
        csv__iscomplete::spec(),
        csv__join::spec(),
        csv__joinlist::spec(),
        csv__joinmatrix::spec(),
        csv__read2matrix::spec(),
        csv__read2queue::spec(),
        csv__report::spec(),
        csv__split::spec(),
        csv__split2matrix::spec(),
        csv__split2queue::spec(),
        csv__writematrix::spec(),
        csv__writequeue::spec(),
        dns__address::spec(),
        dns__cleanup::spec(),
        dns__cname::spec(),
        dns__configure::spec(),
        dns__dump::spec(),
        dns__error::spec(),
        dns__errorcode::spec(),
        dns__name::spec(),
        dns__reset::spec(),
        dns__resolve::spec(),
        dns__result::spec(),
        dns__status::spec(),
        dns__wait::spec(),
        fileutil__appendtofile::spec(),
        fileutil__cat::spec(),
        fileutil__filetype::spec(),
        fileutil__find::spec(),
        fileutil__findbypattern::spec(),
        fileutil__foreachline::spec(),
        fileutil__fullnormalize::spec(),
        fileutil__grep::spec(),
        fileutil__insertintofile::spec(),
        fileutil__install::spec(),
        fileutil__jail::spec(),
        fileutil__lexnormalize::spec(),
        fileutil__maketempdir::spec(),
        fileutil__relative::spec(),
        fileutil__relativeurl::spec(),
        fileutil__removefromfile::spec(),
        fileutil__replaceinfile::spec(),
        fileutil__stripn::spec(),
        fileutil__strippwd::spec(),
        fileutil__tempdir::spec(),
        fileutil__tempdirreset::spec(),
        fileutil__tempfile::spec(),
        fileutil__test::spec(),
        fileutil__touch::spec(),
        fileutil__updateinplace::spec(),
        fileutil__writefile::spec(),
        html__html_entities::spec(),
        html__tagstrip::spec(),
        ip__collapse::spec(),
        ip__contract::spec(),
        ip__equal::spec(),
        ip__is::spec(),
        ip__mask::spec(),
        ip__normalize::spec(),
        ip__prefix::spec(),
        ip__subtract::spec(),
        ip__type::spec(),
        ip__version::spec(),
        json__dict2json::spec(),
        json__json2dict::spec(),
        json__list2json::spec(),
        json__many_json2dict::spec(),
        json__string2json::spec(),
        json__validate::spec(),
        logger__disable::spec(),
        logger__enable::spec(),
        logger__import::spec(),
        logger__init::spec(),
        logger__initnamespace::spec(),
        logger__levels::spec(),
        logger__servicecmd::spec(),
        logger__services::spec(),
        logger__setlevel::spec(),
        logger__walk::spec(),
        math__statistics__analyse_kruskal_wallis::spec(),
        math__statistics__autocorr::spec(),
        math__statistics__basic_stats::spec(),
        math__statistics__control_rchart::spec(),
        math__statistics__control_xbar::spec(),
        math__statistics__corr::spec(),
        math__statistics__crosscorr::spec(),
        math__statistics__filter::spec(),
        math__statistics__group_rank::spec(),
        math__statistics__histogram::spec(),
        math__statistics__histogram_alt::spec(),
        math__statistics__interval_mean_stdev::spec(),
        math__statistics__lillieforsfit::spec(),
        math__statistics__linear_model::spec(),
        math__statistics__linear_residuals::spec(),
        math__statistics__map::spec(),
        math__statistics__max::spec(),
        math__statistics__mean::spec(),
        math__statistics__mean_histogram_limits::spec(),
        math__statistics__median::spec(),
        math__statistics__min::spec(),
        math__statistics__minmax_histogram_limits::spec(),
        math__statistics__number::spec(),
        math__statistics__print_2x2::spec(),
        math__statistics__pstdev::spec(),
        math__statistics__pvar::spec(),
        math__statistics__quantiles::spec(),
        math__statistics__samplescount::spec(),
        math__statistics__spearman_rank::spec(),
        math__statistics__spearman_rank_extended::spec(),
        math__statistics__stdev::spec(),
        math__statistics__t_test_mean::spec(),
        math__statistics__test_2x2::spec(),
        math__statistics__test_anova_f::spec(),
        math__statistics__test_duckworth::spec(),
        math__statistics__test_dunnett::spec(),
        math__statistics__test_kruskal_wallis::spec(),
        math__statistics__test_normal::spec(),
        math__statistics__test_rchart::spec(),
        math__statistics__test_tukey_range::spec(),
        math__statistics__test_wilcoxon::spec(),
        math__statistics__test_xbar::spec(),
        math__statistics__var::spec(),
        md5__md5::spec(),
        mime__buildmessage::spec(),
        mime__copymessage::spec(),
        mime__field_decode::spec(),
        mime__finalize::spec(),
        mime__getbody::spec(),
        mime__getcontenttype::spec(),
        mime__getheader::spec(),
        mime__getproperty::spec(),
        mime__getsize::spec(),
        mime__gettransferencoding::spec(),
        mime__initialize::spec(),
        mime__mapencoding::spec(),
        mime__parseaddress::spec(),
        mime__parsedatetime::spec(),
        mime__reversemapencoding::spec(),
        mime__setheader::spec(),
        mime__uniqueid::spec(),
        mime__word_decode::spec(),
        mime__word_encode::spec(),
        sha1__sha1::spec(),
        sha2__sha256::spec(),
        smtp__sendmessage::spec(),
        snit__compile::spec(),
        snit__macro::spec(),
        snit__method::spec(),
        snit__type::spec(),
        snit__typemethod::spec(),
        snit__widget::spec(),
        snit__widgetadaptor::spec(),
        struct__list::spec(),
        struct__queue::spec(),
        struct__set::spec(),
        struct__stack::spec(),
        textutil__adjust::spec(),
        textutil__blank::spec(),
        textutil__cap::spec(),
        textutil__capeachword::spec(),
        textutil__chop::spec(),
        textutil__indent::spec(),
        textutil__longestcommonprefix::spec(),
        textutil__longestcommonprefixlist::spec(),
        textutil__splitn::spec(),
        textutil__splitx::spec(),
        textutil__strrepeat::spec(),
        textutil__tabify::spec(),
        textutil__tabify2::spec(),
        textutil__tail::spec(),
        textutil__trim::spec(),
        textutil__trimemptyheading::spec(),
        textutil__trimleft::spec(),
        textutil__trimprefix::spec(),
        textutil__trimright::spec(),
        textutil__uncap::spec(),
        textutil__undent::spec(),
        textutil__untabify::spec(),
        textutil__untabify2::spec(),
        uri__canonicalize::spec(),
        uri__geturl::spec(),
        uri__isrelative::spec(),
        uri__join::spec(),
        uri__register::spec(),
        uri__resolve::spec(),
        uri__setquirkoption::spec(),
        uri__split::spec(),
        uuid__uuid::spec(),
        yaml__dict2yaml::spec(),
        yaml__huddle2yaml::spec(),
        yaml__list2yaml::spec(),
        yaml__setoptions::spec(),
        yaml__yaml2dict::spec(),
        yaml__yaml2huddle::spec(),
    ]
}
