decl(var('add', empty, public, false, num), any, fn([],[[var('a'),num], [var('b'),num]],num,sequence([]))),
decl(var('add', empty, public, false, int), any, fn([],[[var('a'),int], [var('b'),int]],int,sequence([]))),
decl(var('map', empty, public, false, tarray(ind('n'), gen('t'))), any, fn([],[[var('a'),tarray(ind('n'), gen('t'))], [var('f'),tfn([], [gen('t')], gen('u'))]],tarray(ind('n'), gen('u')),sequence([]))),
decl(var('incr', empty, public, false, num), any, fn([],[[var('a'),num]],num,sequence([]))),
decl(var('seq', empty, public, false, ind('i')), any, fn([],[[var('a'),ind('i')], [var('b'),ind('j')], [var('c'),ind('k')]],tarray(add(division(minus(ind('j'), ind('i')), ind('k')), 1), int),sequence([empty]))),
decl(var('filter', empty, private, false, tarray(ind('n'), gen('t'))), any, fn([],[[var('n'),tarray(ind('n'), gen('t'))]],tarray(ind('m'), gen('t')),sequence([]))),
decl(var('length', empty, private, false, tarray(ind('n'), gen('t'))), any, fn([],[[var('a'),tarray(ind('n'), gen('t'))]],int,sequence([]))),
decl(var('rev', empty, public, false, tarray(ind('n'), gen('t'))), any, fn([],[[var('a'),tarray(ind('n'), gen('t'))]],tarray(ind('n'), gen('t')),sequence([empty]))),
decl(var('mean', empty, private, false, tarray(ind('n'), int)), any, fn([],[[var('a'),tarray(ind('n'), int)]],num,sequence([empty]))),
decl(var('mean', empty, private, false, tarray(ind('n'), num)), any, fn([],[[var('a'),tarray(ind('n'), num)]],num,sequence([empty]))),
decl(var('sd', empty, private, false, tarray(ind('n'), int)), any, fn([],[[var('a'),tarray(ind('n'), int)]],num,sequence([empty]))),
decl(var('print', empty, private, false, chars), any, fn([],[[var('a'),chars]],ttag('NA', any),sequence([empty]))),
decl(var('paste', empty, private, false, chars), any, fn([],[[var('a'),chars], [var('b'),chars]],chars,sequence([empty]))),
decl(var('class', empty, private, false, gen('t')), any, fn([],[[var('a'),gen('t')]],chars,sequence([empty]))),
decl(var('max', empty, private, false, tarray(ind('n'), int)), any, fn([],[[var('a'),tarray(ind('n'), int)]],num,sequence([empty]))),
decl(var('max', empty, private, false, tarray(ind('n'), num)), any, fn([],[[var('a'),tarray(ind('n'), num)]],num,sequence([empty]))),
decl(var('min', empty, private, false, tarray(ind('n'), int)), any, fn([],[[var('a'),tarray(ind('n'), int)]],num,sequence([empty]))),
decl(var('min', empty, private, false, tarray(ind('n'), num)), any, fn([],[[var('a'),tarray(ind('n'), num)]],num,sequence([empty]))),
decl(var('sqrt', empty, private, false, int), any, fn([],[[var('a'),int]],num,sequence([empty]))),
decl(var('sqrt', empty, private, false, num), any, fn([],[[var('a'),num]],num,sequence([empty]))),
decl(var('abs', empty, private, false, num), any, fn([],[[var('a'),num]],int,sequence([empty]))),
decl(var('ceiling', empty, private, false, num), any, fn([],[[var('a'),num]],int,sequence([empty]))),
decl(var('floor', empty, private, false, num), any, fn([],[[var('a'),num]],int,sequence([empty]))),
decl(var('cat', empty, private, false, num), any, fn([],[[var('a'),chars]],chars,sequence([empty]))),
decl(var('grepl', empty, private, false, num), any, fn([],[[var('a'),chars],[var('b'),chars]],bool,sequence([empty]))),
decl(var('rep', empty, private, false, tarray(ind('n'), gen('t'))), any, fn([],[[var('a'),tarray(ind('n'), gen('t'))], [var('b'),ind('m')]],tarray(mul(ind('n'), ind('m')), gen('t')),sequence([empty]))),
decl(var('sort', empty, private, false, tarray(ind('n'), gen('t'))), any, fn([],[[var('a'),tarray(ind('n'), gen('t'))]],tarray(ind('n'), gen('t')),sequence([empty]))),
decl(var('c', empty, private, false, tarray(ind('n'), gen('t'))), any, fn([],[[var('a'),tarray(ind('n'), gen('t'))], [var('b'),tarray(ind('m'), gen('t'))]],tarray(add(ind('n'), ind('m')), gen('t')),sequence([empty]))),
decl(var('median', empty, private, false, tarray(ind('n'), gen('t'))), any, fn([],[[var('a'),tarray(ind('n'), gen('t'))]],gen('t'),sequence([empty]))),
decl(var('mode', empty, private, false, tarray(ind('n'), gen('t'))), any, fn([],[[var('a'),tarray(ind('n'), gen('t'))]],gen('t'),sequence([empty])))
