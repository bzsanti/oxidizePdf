//! PDF simple-font code-to-Unicode tables (ISO 32000-1 Annex D.2–D.6).
//!
//! Copyright The Apache Software Foundation and Adobe. See the notices in
//! `pdf_encoding_licenses/` (Apache-2.0 and Adobe BSD terms).
//! Adapted into Rust tables with explicit undefined-code recovery.
//!
//! Transcribed from the independently frozen PDFBox 3.0.5 / Adobe AGL
//! references documented in tests/fixtures/text_contracts/provenance.json.
//! Glyph names retain Adobe AGL mappings, including legacy PUA. Duplicate
//! space and hyphen positions retain NBSP/soft hyphen (U+00A0/U+00AD).
//! Unassigned codes recover as U+FFFD; this is extraction policy, not a
//! normative Unicode assignment. These tables are distinct from OS codecs.

pub(super) const STANDARD: [char; 256] = [
    '\u{FFFD}', // 00 /-
    '\u{FFFD}', // 01 /-
    '\u{FFFD}', // 02 /-
    '\u{FFFD}', // 03 /-
    '\u{FFFD}', // 04 /-
    '\u{FFFD}', // 05 /-
    '\u{FFFD}', // 06 /-
    '\u{FFFD}', // 07 /-
    '\u{FFFD}', // 08 /-
    '\u{FFFD}', // 09 /-
    '\u{FFFD}', // 0A /-
    '\u{FFFD}', // 0B /-
    '\u{FFFD}', // 0C /-
    '\u{FFFD}', // 0D /-
    '\u{FFFD}', // 0E /-
    '\u{FFFD}', // 0F /-
    '\u{FFFD}', // 10 /-
    '\u{FFFD}', // 11 /-
    '\u{FFFD}', // 12 /-
    '\u{FFFD}', // 13 /-
    '\u{FFFD}', // 14 /-
    '\u{FFFD}', // 15 /-
    '\u{FFFD}', // 16 /-
    '\u{FFFD}', // 17 /-
    '\u{FFFD}', // 18 /-
    '\u{FFFD}', // 19 /-
    '\u{FFFD}', // 1A /-
    '\u{FFFD}', // 1B /-
    '\u{FFFD}', // 1C /-
    '\u{FFFD}', // 1D /-
    '\u{FFFD}', // 1E /-
    '\u{FFFD}', // 1F /-
    '\u{0020}', // 20 /space
    '\u{0021}', // 21 /exclam
    '\u{0022}', // 22 /quotedbl
    '\u{0023}', // 23 /numbersign
    '\u{0024}', // 24 /dollar
    '\u{0025}', // 25 /percent
    '\u{0026}', // 26 /ampersand
    '\u{2019}', // 27 /quoteright
    '\u{0028}', // 28 /parenleft
    '\u{0029}', // 29 /parenright
    '\u{002A}', // 2A /asterisk
    '\u{002B}', // 2B /plus
    '\u{002C}', // 2C /comma
    '\u{002D}', // 2D /hyphen
    '\u{002E}', // 2E /period
    '\u{002F}', // 2F /slash
    '\u{0030}', // 30 /zero
    '\u{0031}', // 31 /one
    '\u{0032}', // 32 /two
    '\u{0033}', // 33 /three
    '\u{0034}', // 34 /four
    '\u{0035}', // 35 /five
    '\u{0036}', // 36 /six
    '\u{0037}', // 37 /seven
    '\u{0038}', // 38 /eight
    '\u{0039}', // 39 /nine
    '\u{003A}', // 3A /colon
    '\u{003B}', // 3B /semicolon
    '\u{003C}', // 3C /less
    '\u{003D}', // 3D /equal
    '\u{003E}', // 3E /greater
    '\u{003F}', // 3F /question
    '\u{0040}', // 40 /at
    '\u{0041}', // 41 /A
    '\u{0042}', // 42 /B
    '\u{0043}', // 43 /C
    '\u{0044}', // 44 /D
    '\u{0045}', // 45 /E
    '\u{0046}', // 46 /F
    '\u{0047}', // 47 /G
    '\u{0048}', // 48 /H
    '\u{0049}', // 49 /I
    '\u{004A}', // 4A /J
    '\u{004B}', // 4B /K
    '\u{004C}', // 4C /L
    '\u{004D}', // 4D /M
    '\u{004E}', // 4E /N
    '\u{004F}', // 4F /O
    '\u{0050}', // 50 /P
    '\u{0051}', // 51 /Q
    '\u{0052}', // 52 /R
    '\u{0053}', // 53 /S
    '\u{0054}', // 54 /T
    '\u{0055}', // 55 /U
    '\u{0056}', // 56 /V
    '\u{0057}', // 57 /W
    '\u{0058}', // 58 /X
    '\u{0059}', // 59 /Y
    '\u{005A}', // 5A /Z
    '\u{005B}', // 5B /bracketleft
    '\u{005C}', // 5C /backslash
    '\u{005D}', // 5D /bracketright
    '\u{005E}', // 5E /asciicircum
    '\u{005F}', // 5F /underscore
    '\u{2018}', // 60 /quoteleft
    '\u{0061}', // 61 /a
    '\u{0062}', // 62 /b
    '\u{0063}', // 63 /c
    '\u{0064}', // 64 /d
    '\u{0065}', // 65 /e
    '\u{0066}', // 66 /f
    '\u{0067}', // 67 /g
    '\u{0068}', // 68 /h
    '\u{0069}', // 69 /i
    '\u{006A}', // 6A /j
    '\u{006B}', // 6B /k
    '\u{006C}', // 6C /l
    '\u{006D}', // 6D /m
    '\u{006E}', // 6E /n
    '\u{006F}', // 6F /o
    '\u{0070}', // 70 /p
    '\u{0071}', // 71 /q
    '\u{0072}', // 72 /r
    '\u{0073}', // 73 /s
    '\u{0074}', // 74 /t
    '\u{0075}', // 75 /u
    '\u{0076}', // 76 /v
    '\u{0077}', // 77 /w
    '\u{0078}', // 78 /x
    '\u{0079}', // 79 /y
    '\u{007A}', // 7A /z
    '\u{007B}', // 7B /braceleft
    '\u{007C}', // 7C /bar
    '\u{007D}', // 7D /braceright
    '\u{007E}', // 7E /asciitilde
    '\u{FFFD}', // 7F /-
    '\u{FFFD}', // 80 /-
    '\u{FFFD}', // 81 /-
    '\u{FFFD}', // 82 /-
    '\u{FFFD}', // 83 /-
    '\u{FFFD}', // 84 /-
    '\u{FFFD}', // 85 /-
    '\u{FFFD}', // 86 /-
    '\u{FFFD}', // 87 /-
    '\u{FFFD}', // 88 /-
    '\u{FFFD}', // 89 /-
    '\u{FFFD}', // 8A /-
    '\u{FFFD}', // 8B /-
    '\u{FFFD}', // 8C /-
    '\u{FFFD}', // 8D /-
    '\u{FFFD}', // 8E /-
    '\u{FFFD}', // 8F /-
    '\u{FFFD}', // 90 /-
    '\u{FFFD}', // 91 /-
    '\u{FFFD}', // 92 /-
    '\u{FFFD}', // 93 /-
    '\u{FFFD}', // 94 /-
    '\u{FFFD}', // 95 /-
    '\u{FFFD}', // 96 /-
    '\u{FFFD}', // 97 /-
    '\u{FFFD}', // 98 /-
    '\u{FFFD}', // 99 /-
    '\u{FFFD}', // 9A /-
    '\u{FFFD}', // 9B /-
    '\u{FFFD}', // 9C /-
    '\u{FFFD}', // 9D /-
    '\u{FFFD}', // 9E /-
    '\u{FFFD}', // 9F /-
    '\u{FFFD}', // A0 /-
    '\u{00A1}', // A1 /exclamdown
    '\u{00A2}', // A2 /cent
    '\u{00A3}', // A3 /sterling
    '\u{2044}', // A4 /fraction
    '\u{00A5}', // A5 /yen
    '\u{0192}', // A6 /florin
    '\u{00A7}', // A7 /section
    '\u{00A4}', // A8 /currency
    '\u{0027}', // A9 /quotesingle
    '\u{201C}', // AA /quotedblleft
    '\u{00AB}', // AB /guillemotleft
    '\u{2039}', // AC /guilsinglleft
    '\u{203A}', // AD /guilsinglright
    '\u{FB01}', // AE /fi
    '\u{FB02}', // AF /fl
    '\u{FFFD}', // B0 /-
    '\u{2013}', // B1 /endash
    '\u{2020}', // B2 /dagger
    '\u{2021}', // B3 /daggerdbl
    '\u{00B7}', // B4 /periodcentered
    '\u{FFFD}', // B5 /-
    '\u{00B6}', // B6 /paragraph
    '\u{2022}', // B7 /bullet
    '\u{201A}', // B8 /quotesinglbase
    '\u{201E}', // B9 /quotedblbase
    '\u{201D}', // BA /quotedblright
    '\u{00BB}', // BB /guillemotright
    '\u{2026}', // BC /ellipsis
    '\u{2030}', // BD /perthousand
    '\u{FFFD}', // BE /-
    '\u{00BF}', // BF /questiondown
    '\u{FFFD}', // C0 /-
    '\u{0060}', // C1 /grave
    '\u{00B4}', // C2 /acute
    '\u{02C6}', // C3 /circumflex
    '\u{02DC}', // C4 /tilde
    '\u{00AF}', // C5 /macron
    '\u{02D8}', // C6 /breve
    '\u{02D9}', // C7 /dotaccent
    '\u{00A8}', // C8 /dieresis
    '\u{FFFD}', // C9 /-
    '\u{02DA}', // CA /ring
    '\u{00B8}', // CB /cedilla
    '\u{FFFD}', // CC /-
    '\u{02DD}', // CD /hungarumlaut
    '\u{02DB}', // CE /ogonek
    '\u{02C7}', // CF /caron
    '\u{2014}', // D0 /emdash
    '\u{FFFD}', // D1 /-
    '\u{FFFD}', // D2 /-
    '\u{FFFD}', // D3 /-
    '\u{FFFD}', // D4 /-
    '\u{FFFD}', // D5 /-
    '\u{FFFD}', // D6 /-
    '\u{FFFD}', // D7 /-
    '\u{FFFD}', // D8 /-
    '\u{FFFD}', // D9 /-
    '\u{FFFD}', // DA /-
    '\u{FFFD}', // DB /-
    '\u{FFFD}', // DC /-
    '\u{FFFD}', // DD /-
    '\u{FFFD}', // DE /-
    '\u{FFFD}', // DF /-
    '\u{FFFD}', // E0 /-
    '\u{00C6}', // E1 /AE
    '\u{FFFD}', // E2 /-
    '\u{00AA}', // E3 /ordfeminine
    '\u{FFFD}', // E4 /-
    '\u{FFFD}', // E5 /-
    '\u{FFFD}', // E6 /-
    '\u{FFFD}', // E7 /-
    '\u{0141}', // E8 /Lslash
    '\u{00D8}', // E9 /Oslash
    '\u{0152}', // EA /OE
    '\u{00BA}', // EB /ordmasculine
    '\u{FFFD}', // EC /-
    '\u{FFFD}', // ED /-
    '\u{FFFD}', // EE /-
    '\u{FFFD}', // EF /-
    '\u{FFFD}', // F0 /-
    '\u{00E6}', // F1 /ae
    '\u{FFFD}', // F2 /-
    '\u{FFFD}', // F3 /-
    '\u{FFFD}', // F4 /-
    '\u{0131}', // F5 /dotlessi
    '\u{FFFD}', // F6 /-
    '\u{FFFD}', // F7 /-
    '\u{0142}', // F8 /lslash
    '\u{00F8}', // F9 /oslash
    '\u{0153}', // FA /oe
    '\u{00DF}', // FB /germandbls
    '\u{FFFD}', // FC /-
    '\u{FFFD}', // FD /-
    '\u{FFFD}', // FE /-
    '\u{FFFD}', // FF /-
];

pub(super) const WIN_ANSI: [char; 256] = [
    '\u{FFFD}', // 00 /-
    '\u{FFFD}', // 01 /-
    '\u{FFFD}', // 02 /-
    '\u{FFFD}', // 03 /-
    '\u{FFFD}', // 04 /-
    '\u{FFFD}', // 05 /-
    '\u{FFFD}', // 06 /-
    '\u{FFFD}', // 07 /-
    '\u{FFFD}', // 08 /-
    '\u{FFFD}', // 09 /-
    '\u{FFFD}', // 0A /-
    '\u{FFFD}', // 0B /-
    '\u{FFFD}', // 0C /-
    '\u{FFFD}', // 0D /-
    '\u{FFFD}', // 0E /-
    '\u{FFFD}', // 0F /-
    '\u{FFFD}', // 10 /-
    '\u{FFFD}', // 11 /-
    '\u{FFFD}', // 12 /-
    '\u{FFFD}', // 13 /-
    '\u{FFFD}', // 14 /-
    '\u{FFFD}', // 15 /-
    '\u{FFFD}', // 16 /-
    '\u{FFFD}', // 17 /-
    '\u{FFFD}', // 18 /-
    '\u{FFFD}', // 19 /-
    '\u{FFFD}', // 1A /-
    '\u{FFFD}', // 1B /-
    '\u{FFFD}', // 1C /-
    '\u{FFFD}', // 1D /-
    '\u{FFFD}', // 1E /-
    '\u{FFFD}', // 1F /-
    '\u{0020}', // 20 /space
    '\u{0021}', // 21 /exclam
    '\u{0022}', // 22 /quotedbl
    '\u{0023}', // 23 /numbersign
    '\u{0024}', // 24 /dollar
    '\u{0025}', // 25 /percent
    '\u{0026}', // 26 /ampersand
    '\u{0027}', // 27 /quotesingle
    '\u{0028}', // 28 /parenleft
    '\u{0029}', // 29 /parenright
    '\u{002A}', // 2A /asterisk
    '\u{002B}', // 2B /plus
    '\u{002C}', // 2C /comma
    '\u{002D}', // 2D /hyphen
    '\u{002E}', // 2E /period
    '\u{002F}', // 2F /slash
    '\u{0030}', // 30 /zero
    '\u{0031}', // 31 /one
    '\u{0032}', // 32 /two
    '\u{0033}', // 33 /three
    '\u{0034}', // 34 /four
    '\u{0035}', // 35 /five
    '\u{0036}', // 36 /six
    '\u{0037}', // 37 /seven
    '\u{0038}', // 38 /eight
    '\u{0039}', // 39 /nine
    '\u{003A}', // 3A /colon
    '\u{003B}', // 3B /semicolon
    '\u{003C}', // 3C /less
    '\u{003D}', // 3D /equal
    '\u{003E}', // 3E /greater
    '\u{003F}', // 3F /question
    '\u{0040}', // 40 /at
    '\u{0041}', // 41 /A
    '\u{0042}', // 42 /B
    '\u{0043}', // 43 /C
    '\u{0044}', // 44 /D
    '\u{0045}', // 45 /E
    '\u{0046}', // 46 /F
    '\u{0047}', // 47 /G
    '\u{0048}', // 48 /H
    '\u{0049}', // 49 /I
    '\u{004A}', // 4A /J
    '\u{004B}', // 4B /K
    '\u{004C}', // 4C /L
    '\u{004D}', // 4D /M
    '\u{004E}', // 4E /N
    '\u{004F}', // 4F /O
    '\u{0050}', // 50 /P
    '\u{0051}', // 51 /Q
    '\u{0052}', // 52 /R
    '\u{0053}', // 53 /S
    '\u{0054}', // 54 /T
    '\u{0055}', // 55 /U
    '\u{0056}', // 56 /V
    '\u{0057}', // 57 /W
    '\u{0058}', // 58 /X
    '\u{0059}', // 59 /Y
    '\u{005A}', // 5A /Z
    '\u{005B}', // 5B /bracketleft
    '\u{005C}', // 5C /backslash
    '\u{005D}', // 5D /bracketright
    '\u{005E}', // 5E /asciicircum
    '\u{005F}', // 5F /underscore
    '\u{0060}', // 60 /grave
    '\u{0061}', // 61 /a
    '\u{0062}', // 62 /b
    '\u{0063}', // 63 /c
    '\u{0064}', // 64 /d
    '\u{0065}', // 65 /e
    '\u{0066}', // 66 /f
    '\u{0067}', // 67 /g
    '\u{0068}', // 68 /h
    '\u{0069}', // 69 /i
    '\u{006A}', // 6A /j
    '\u{006B}', // 6B /k
    '\u{006C}', // 6C /l
    '\u{006D}', // 6D /m
    '\u{006E}', // 6E /n
    '\u{006F}', // 6F /o
    '\u{0070}', // 70 /p
    '\u{0071}', // 71 /q
    '\u{0072}', // 72 /r
    '\u{0073}', // 73 /s
    '\u{0074}', // 74 /t
    '\u{0075}', // 75 /u
    '\u{0076}', // 76 /v
    '\u{0077}', // 77 /w
    '\u{0078}', // 78 /x
    '\u{0079}', // 79 /y
    '\u{007A}', // 7A /z
    '\u{007B}', // 7B /braceleft
    '\u{007C}', // 7C /bar
    '\u{007D}', // 7D /braceright
    '\u{007E}', // 7E /asciitilde
    '\u{2022}', // 7F /bullet
    '\u{20AC}', // 80 /Euro
    '\u{2022}', // 81 /bullet
    '\u{201A}', // 82 /quotesinglbase
    '\u{0192}', // 83 /florin
    '\u{201E}', // 84 /quotedblbase
    '\u{2026}', // 85 /ellipsis
    '\u{2020}', // 86 /dagger
    '\u{2021}', // 87 /daggerdbl
    '\u{02C6}', // 88 /circumflex
    '\u{2030}', // 89 /perthousand
    '\u{0160}', // 8A /Scaron
    '\u{2039}', // 8B /guilsinglleft
    '\u{0152}', // 8C /OE
    '\u{2022}', // 8D /bullet
    '\u{017D}', // 8E /Zcaron
    '\u{2022}', // 8F /bullet
    '\u{2022}', // 90 /bullet
    '\u{2018}', // 91 /quoteleft
    '\u{2019}', // 92 /quoteright
    '\u{201C}', // 93 /quotedblleft
    '\u{201D}', // 94 /quotedblright
    '\u{2022}', // 95 /bullet
    '\u{2013}', // 96 /endash
    '\u{2014}', // 97 /emdash
    '\u{02DC}', // 98 /tilde
    '\u{2122}', // 99 /trademark
    '\u{0161}', // 9A /scaron
    '\u{203A}', // 9B /guilsinglright
    '\u{0153}', // 9C /oe
    '\u{2022}', // 9D /bullet
    '\u{017E}', // 9E /zcaron
    '\u{0178}', // 9F /Ydieresis
    '\u{00A0}', // A0 /nbspace
    '\u{00A1}', // A1 /exclamdown
    '\u{00A2}', // A2 /cent
    '\u{00A3}', // A3 /sterling
    '\u{00A4}', // A4 /currency
    '\u{00A5}', // A5 /yen
    '\u{00A6}', // A6 /brokenbar
    '\u{00A7}', // A7 /section
    '\u{00A8}', // A8 /dieresis
    '\u{00A9}', // A9 /copyright
    '\u{00AA}', // AA /ordfeminine
    '\u{00AB}', // AB /guillemotleft
    '\u{00AC}', // AC /logicalnot
    '\u{00AD}', // AD /sfthyphen
    '\u{00AE}', // AE /registered
    '\u{00AF}', // AF /macron
    '\u{00B0}', // B0 /degree
    '\u{00B1}', // B1 /plusminus
    '\u{00B2}', // B2 /twosuperior
    '\u{00B3}', // B3 /threesuperior
    '\u{00B4}', // B4 /acute
    '\u{00B5}', // B5 /mu
    '\u{00B6}', // B6 /paragraph
    '\u{00B7}', // B7 /periodcentered
    '\u{00B8}', // B8 /cedilla
    '\u{00B9}', // B9 /onesuperior
    '\u{00BA}', // BA /ordmasculine
    '\u{00BB}', // BB /guillemotright
    '\u{00BC}', // BC /onequarter
    '\u{00BD}', // BD /onehalf
    '\u{00BE}', // BE /threequarters
    '\u{00BF}', // BF /questiondown
    '\u{00C0}', // C0 /Agrave
    '\u{00C1}', // C1 /Aacute
    '\u{00C2}', // C2 /Acircumflex
    '\u{00C3}', // C3 /Atilde
    '\u{00C4}', // C4 /Adieresis
    '\u{00C5}', // C5 /Aring
    '\u{00C6}', // C6 /AE
    '\u{00C7}', // C7 /Ccedilla
    '\u{00C8}', // C8 /Egrave
    '\u{00C9}', // C9 /Eacute
    '\u{00CA}', // CA /Ecircumflex
    '\u{00CB}', // CB /Edieresis
    '\u{00CC}', // CC /Igrave
    '\u{00CD}', // CD /Iacute
    '\u{00CE}', // CE /Icircumflex
    '\u{00CF}', // CF /Idieresis
    '\u{00D0}', // D0 /Eth
    '\u{00D1}', // D1 /Ntilde
    '\u{00D2}', // D2 /Ograve
    '\u{00D3}', // D3 /Oacute
    '\u{00D4}', // D4 /Ocircumflex
    '\u{00D5}', // D5 /Otilde
    '\u{00D6}', // D6 /Odieresis
    '\u{00D7}', // D7 /multiply
    '\u{00D8}', // D8 /Oslash
    '\u{00D9}', // D9 /Ugrave
    '\u{00DA}', // DA /Uacute
    '\u{00DB}', // DB /Ucircumflex
    '\u{00DC}', // DC /Udieresis
    '\u{00DD}', // DD /Yacute
    '\u{00DE}', // DE /Thorn
    '\u{00DF}', // DF /germandbls
    '\u{00E0}', // E0 /agrave
    '\u{00E1}', // E1 /aacute
    '\u{00E2}', // E2 /acircumflex
    '\u{00E3}', // E3 /atilde
    '\u{00E4}', // E4 /adieresis
    '\u{00E5}', // E5 /aring
    '\u{00E6}', // E6 /ae
    '\u{00E7}', // E7 /ccedilla
    '\u{00E8}', // E8 /egrave
    '\u{00E9}', // E9 /eacute
    '\u{00EA}', // EA /ecircumflex
    '\u{00EB}', // EB /edieresis
    '\u{00EC}', // EC /igrave
    '\u{00ED}', // ED /iacute
    '\u{00EE}', // EE /icircumflex
    '\u{00EF}', // EF /idieresis
    '\u{00F0}', // F0 /eth
    '\u{00F1}', // F1 /ntilde
    '\u{00F2}', // F2 /ograve
    '\u{00F3}', // F3 /oacute
    '\u{00F4}', // F4 /ocircumflex
    '\u{00F5}', // F5 /otilde
    '\u{00F6}', // F6 /odieresis
    '\u{00F7}', // F7 /divide
    '\u{00F8}', // F8 /oslash
    '\u{00F9}', // F9 /ugrave
    '\u{00FA}', // FA /uacute
    '\u{00FB}', // FB /ucircumflex
    '\u{00FC}', // FC /udieresis
    '\u{00FD}', // FD /yacute
    '\u{00FE}', // FE /thorn
    '\u{00FF}', // FF /ydieresis
];

pub(super) const MAC_EXPERT: [char; 256] = [
    '\u{FFFD}', // 00 /-
    '\u{FFFD}', // 01 /-
    '\u{FFFD}', // 02 /-
    '\u{FFFD}', // 03 /-
    '\u{FFFD}', // 04 /-
    '\u{FFFD}', // 05 /-
    '\u{FFFD}', // 06 /-
    '\u{FFFD}', // 07 /-
    '\u{FFFD}', // 08 /-
    '\u{FFFD}', // 09 /-
    '\u{FFFD}', // 0A /-
    '\u{FFFD}', // 0B /-
    '\u{FFFD}', // 0C /-
    '\u{FFFD}', // 0D /-
    '\u{FFFD}', // 0E /-
    '\u{FFFD}', // 0F /-
    '\u{FFFD}', // 10 /-
    '\u{FFFD}', // 11 /-
    '\u{FFFD}', // 12 /-
    '\u{FFFD}', // 13 /-
    '\u{FFFD}', // 14 /-
    '\u{FFFD}', // 15 /-
    '\u{FFFD}', // 16 /-
    '\u{FFFD}', // 17 /-
    '\u{FFFD}', // 18 /-
    '\u{FFFD}', // 19 /-
    '\u{FFFD}', // 1A /-
    '\u{FFFD}', // 1B /-
    '\u{FFFD}', // 1C /-
    '\u{FFFD}', // 1D /-
    '\u{FFFD}', // 1E /-
    '\u{FFFD}', // 1F /-
    '\u{0020}', // 20 /space
    '\u{F721}', // 21 /exclamsmall
    '\u{F6F8}', // 22 /Hungarumlautsmall
    '\u{F7A2}', // 23 /centoldstyle
    '\u{F724}', // 24 /dollaroldstyle
    '\u{F6E4}', // 25 /dollarsuperior
    '\u{F726}', // 26 /ampersandsmall
    '\u{F7B4}', // 27 /Acutesmall
    '\u{207D}', // 28 /parenleftsuperior
    '\u{207E}', // 29 /parenrightsuperior
    '\u{2025}', // 2A /twodotenleader
    '\u{2024}', // 2B /onedotenleader
    '\u{002C}', // 2C /comma
    '\u{002D}', // 2D /hyphen
    '\u{002E}', // 2E /period
    '\u{2044}', // 2F /fraction
    '\u{F730}', // 30 /zerooldstyle
    '\u{F731}', // 31 /oneoldstyle
    '\u{F732}', // 32 /twooldstyle
    '\u{F733}', // 33 /threeoldstyle
    '\u{F734}', // 34 /fouroldstyle
    '\u{F735}', // 35 /fiveoldstyle
    '\u{F736}', // 36 /sixoldstyle
    '\u{F737}', // 37 /sevenoldstyle
    '\u{F738}', // 38 /eightoldstyle
    '\u{F739}', // 39 /nineoldstyle
    '\u{003A}', // 3A /colon
    '\u{003B}', // 3B /semicolon
    '\u{FFFD}', // 3C /-
    '\u{F6DE}', // 3D /threequartersemdash
    '\u{FFFD}', // 3E /-
    '\u{F73F}', // 3F /questionsmall
    '\u{FFFD}', // 40 /-
    '\u{FFFD}', // 41 /-
    '\u{FFFD}', // 42 /-
    '\u{FFFD}', // 43 /-
    '\u{F7F0}', // 44 /Ethsmall
    '\u{FFFD}', // 45 /-
    '\u{FFFD}', // 46 /-
    '\u{00BC}', // 47 /onequarter
    '\u{00BD}', // 48 /onehalf
    '\u{00BE}', // 49 /threequarters
    '\u{215B}', // 4A /oneeighth
    '\u{215C}', // 4B /threeeighths
    '\u{215D}', // 4C /fiveeighths
    '\u{215E}', // 4D /seveneighths
    '\u{2153}', // 4E /onethird
    '\u{2154}', // 4F /twothirds
    '\u{FFFD}', // 50 /-
    '\u{FFFD}', // 51 /-
    '\u{FFFD}', // 52 /-
    '\u{FFFD}', // 53 /-
    '\u{FFFD}', // 54 /-
    '\u{FFFD}', // 55 /-
    '\u{FB00}', // 56 /ff
    '\u{FB01}', // 57 /fi
    '\u{FB02}', // 58 /fl
    '\u{FB03}', // 59 /ffi
    '\u{FB04}', // 5A /ffl
    '\u{208D}', // 5B /parenleftinferior
    '\u{FFFD}', // 5C /-
    '\u{208E}', // 5D /parenrightinferior
    '\u{F6F6}', // 5E /Circumflexsmall
    '\u{F6E5}', // 5F /hypheninferior
    '\u{F760}', // 60 /Gravesmall
    '\u{F761}', // 61 /Asmall
    '\u{F762}', // 62 /Bsmall
    '\u{F763}', // 63 /Csmall
    '\u{F764}', // 64 /Dsmall
    '\u{F765}', // 65 /Esmall
    '\u{F766}', // 66 /Fsmall
    '\u{F767}', // 67 /Gsmall
    '\u{F768}', // 68 /Hsmall
    '\u{F769}', // 69 /Ismall
    '\u{F76A}', // 6A /Jsmall
    '\u{F76B}', // 6B /Ksmall
    '\u{F76C}', // 6C /Lsmall
    '\u{F76D}', // 6D /Msmall
    '\u{F76E}', // 6E /Nsmall
    '\u{F76F}', // 6F /Osmall
    '\u{F770}', // 70 /Psmall
    '\u{F771}', // 71 /Qsmall
    '\u{F772}', // 72 /Rsmall
    '\u{F773}', // 73 /Ssmall
    '\u{F774}', // 74 /Tsmall
    '\u{F775}', // 75 /Usmall
    '\u{F776}', // 76 /Vsmall
    '\u{F777}', // 77 /Wsmall
    '\u{F778}', // 78 /Xsmall
    '\u{F779}', // 79 /Ysmall
    '\u{F77A}', // 7A /Zsmall
    '\u{20A1}', // 7B /colonmonetary
    '\u{F6DC}', // 7C /onefitted
    '\u{F6DD}', // 7D /rupiah
    '\u{F6FE}', // 7E /Tildesmall
    '\u{FFFD}', // 7F /-
    '\u{FFFD}', // 80 /-
    '\u{F6E9}', // 81 /asuperior
    '\u{F6E0}', // 82 /centsuperior
    '\u{FFFD}', // 83 /-
    '\u{FFFD}', // 84 /-
    '\u{FFFD}', // 85 /-
    '\u{FFFD}', // 86 /-
    '\u{F7E1}', // 87 /Aacutesmall
    '\u{F7E0}', // 88 /Agravesmall
    '\u{F7E2}', // 89 /Acircumflexsmall
    '\u{F7E4}', // 8A /Adieresissmall
    '\u{F7E3}', // 8B /Atildesmall
    '\u{F7E5}', // 8C /Aringsmall
    '\u{F7E7}', // 8D /Ccedillasmall
    '\u{F7E9}', // 8E /Eacutesmall
    '\u{F7E8}', // 8F /Egravesmall
    '\u{F7EA}', // 90 /Ecircumflexsmall
    '\u{F7EB}', // 91 /Edieresissmall
    '\u{F7ED}', // 92 /Iacutesmall
    '\u{F7EC}', // 93 /Igravesmall
    '\u{F7EE}', // 94 /Icircumflexsmall
    '\u{F7EF}', // 95 /Idieresissmall
    '\u{F7F1}', // 96 /Ntildesmall
    '\u{F7F3}', // 97 /Oacutesmall
    '\u{F7F2}', // 98 /Ogravesmall
    '\u{F7F4}', // 99 /Ocircumflexsmall
    '\u{F7F6}', // 9A /Odieresissmall
    '\u{F7F5}', // 9B /Otildesmall
    '\u{F7FA}', // 9C /Uacutesmall
    '\u{F7F9}', // 9D /Ugravesmall
    '\u{F7FB}', // 9E /Ucircumflexsmall
    '\u{F7FC}', // 9F /Udieresissmall
    '\u{FFFD}', // A0 /-
    '\u{2078}', // A1 /eightsuperior
    '\u{2084}', // A2 /fourinferior
    '\u{2083}', // A3 /threeinferior
    '\u{2086}', // A4 /sixinferior
    '\u{2088}', // A5 /eightinferior
    '\u{2087}', // A6 /seveninferior
    '\u{F6FD}', // A7 /Scaronsmall
    '\u{FFFD}', // A8 /-
    '\u{F6DF}', // A9 /centinferior
    '\u{2082}', // AA /twoinferior
    '\u{FFFD}', // AB /-
    '\u{F7A8}', // AC /Dieresissmall
    '\u{FFFD}', // AD /-
    '\u{F6F5}', // AE /Caronsmall
    '\u{F6F0}', // AF /osuperior
    '\u{2085}', // B0 /fiveinferior
    '\u{FFFD}', // B1 /-
    '\u{F6E1}', // B2 /commainferior
    '\u{F6E7}', // B3 /periodinferior
    '\u{F7FD}', // B4 /Yacutesmall
    '\u{FFFD}', // B5 /-
    '\u{F6E3}', // B6 /dollarinferior
    '\u{FFFD}', // B7 /-
    '\u{FFFD}', // B8 /-
    '\u{F7FE}', // B9 /Thornsmall
    '\u{FFFD}', // BA /-
    '\u{2089}', // BB /nineinferior
    '\u{2080}', // BC /zeroinferior
    '\u{F6FF}', // BD /Zcaronsmall
    '\u{F7E6}', // BE /AEsmall
    '\u{F7F8}', // BF /Oslashsmall
    '\u{F7BF}', // C0 /questiondownsmall
    '\u{2081}', // C1 /oneinferior
    '\u{F6F9}', // C2 /Lslashsmall
    '\u{FFFD}', // C3 /-
    '\u{FFFD}', // C4 /-
    '\u{FFFD}', // C5 /-
    '\u{FFFD}', // C6 /-
    '\u{FFFD}', // C7 /-
    '\u{FFFD}', // C8 /-
    '\u{F7B8}', // C9 /Cedillasmall
    '\u{FFFD}', // CA /-
    '\u{FFFD}', // CB /-
    '\u{FFFD}', // CC /-
    '\u{FFFD}', // CD /-
    '\u{FFFD}', // CE /-
    '\u{F6FA}', // CF /OEsmall
    '\u{2012}', // D0 /figuredash
    '\u{F6E6}', // D1 /hyphensuperior
    '\u{FFFD}', // D2 /-
    '\u{FFFD}', // D3 /-
    '\u{FFFD}', // D4 /-
    '\u{FFFD}', // D5 /-
    '\u{F7A1}', // D6 /exclamdownsmall
    '\u{FFFD}', // D7 /-
    '\u{F7FF}', // D8 /Ydieresissmall
    '\u{FFFD}', // D9 /-
    '\u{00B9}', // DA /onesuperior
    '\u{00B2}', // DB /twosuperior
    '\u{00B3}', // DC /threesuperior
    '\u{2074}', // DD /foursuperior
    '\u{2075}', // DE /fivesuperior
    '\u{2076}', // DF /sixsuperior
    '\u{2077}', // E0 /sevensuperior
    '\u{2079}', // E1 /ninesuperior
    '\u{2070}', // E2 /zerosuperior
    '\u{FFFD}', // E3 /-
    '\u{F6EC}', // E4 /esuperior
    '\u{F6F1}', // E5 /rsuperior
    '\u{F6F3}', // E6 /tsuperior
    '\u{FFFD}', // E7 /-
    '\u{FFFD}', // E8 /-
    '\u{F6ED}', // E9 /isuperior
    '\u{F6F2}', // EA /ssuperior
    '\u{F6EB}', // EB /dsuperior
    '\u{FFFD}', // EC /-
    '\u{FFFD}', // ED /-
    '\u{FFFD}', // EE /-
    '\u{FFFD}', // EF /-
    '\u{FFFD}', // F0 /-
    '\u{F6EE}', // F1 /lsuperior
    '\u{F6FB}', // F2 /Ogoneksmall
    '\u{F6F4}', // F3 /Brevesmall
    '\u{F7AF}', // F4 /Macronsmall
    '\u{F6EA}', // F5 /bsuperior
    '\u{207F}', // F6 /nsuperior
    '\u{F6EF}', // F7 /msuperior
    '\u{F6E2}', // F8 /commasuperior
    '\u{F6E8}', // F9 /periodsuperior
    '\u{F6F7}', // FA /Dotaccentsmall
    '\u{F6FC}', // FB /Ringsmall
    '\u{FFFD}', // FC /-
    '\u{FFFD}', // FD /-
    '\u{FFFD}', // FE /-
    '\u{FFFD}', // FF /-
];

pub(super) const SYMBOL: [char; 256] = [
    '\u{FFFD}', // 00 /-
    '\u{FFFD}', // 01 /-
    '\u{FFFD}', // 02 /-
    '\u{FFFD}', // 03 /-
    '\u{FFFD}', // 04 /-
    '\u{FFFD}', // 05 /-
    '\u{FFFD}', // 06 /-
    '\u{FFFD}', // 07 /-
    '\u{FFFD}', // 08 /-
    '\u{FFFD}', // 09 /-
    '\u{FFFD}', // 0A /-
    '\u{FFFD}', // 0B /-
    '\u{FFFD}', // 0C /-
    '\u{FFFD}', // 0D /-
    '\u{FFFD}', // 0E /-
    '\u{FFFD}', // 0F /-
    '\u{FFFD}', // 10 /-
    '\u{FFFD}', // 11 /-
    '\u{FFFD}', // 12 /-
    '\u{FFFD}', // 13 /-
    '\u{FFFD}', // 14 /-
    '\u{FFFD}', // 15 /-
    '\u{FFFD}', // 16 /-
    '\u{FFFD}', // 17 /-
    '\u{FFFD}', // 18 /-
    '\u{FFFD}', // 19 /-
    '\u{FFFD}', // 1A /-
    '\u{FFFD}', // 1B /-
    '\u{FFFD}', // 1C /-
    '\u{FFFD}', // 1D /-
    '\u{FFFD}', // 1E /-
    '\u{FFFD}', // 1F /-
    '\u{0020}', // 20 /space
    '\u{0021}', // 21 /exclam
    '\u{2200}', // 22 /universal
    '\u{0023}', // 23 /numbersign
    '\u{2203}', // 24 /existential
    '\u{0025}', // 25 /percent
    '\u{0026}', // 26 /ampersand
    '\u{220B}', // 27 /suchthat
    '\u{0028}', // 28 /parenleft
    '\u{0029}', // 29 /parenright
    '\u{2217}', // 2A /asteriskmath
    '\u{002B}', // 2B /plus
    '\u{002C}', // 2C /comma
    '\u{2212}', // 2D /minus
    '\u{002E}', // 2E /period
    '\u{002F}', // 2F /slash
    '\u{0030}', // 30 /zero
    '\u{0031}', // 31 /one
    '\u{0032}', // 32 /two
    '\u{0033}', // 33 /three
    '\u{0034}', // 34 /four
    '\u{0035}', // 35 /five
    '\u{0036}', // 36 /six
    '\u{0037}', // 37 /seven
    '\u{0038}', // 38 /eight
    '\u{0039}', // 39 /nine
    '\u{003A}', // 3A /colon
    '\u{003B}', // 3B /semicolon
    '\u{003C}', // 3C /less
    '\u{003D}', // 3D /equal
    '\u{003E}', // 3E /greater
    '\u{003F}', // 3F /question
    '\u{2245}', // 40 /congruent
    '\u{0391}', // 41 /Alpha
    '\u{0392}', // 42 /Beta
    '\u{03A7}', // 43 /Chi
    '\u{2206}', // 44 /Delta
    '\u{0395}', // 45 /Epsilon
    '\u{03A6}', // 46 /Phi
    '\u{0393}', // 47 /Gamma
    '\u{0397}', // 48 /Eta
    '\u{0399}', // 49 /Iota
    '\u{03D1}', // 4A /theta1
    '\u{039A}', // 4B /Kappa
    '\u{039B}', // 4C /Lambda
    '\u{039C}', // 4D /Mu
    '\u{039D}', // 4E /Nu
    '\u{039F}', // 4F /Omicron
    '\u{03A0}', // 50 /Pi
    '\u{0398}', // 51 /Theta
    '\u{03A1}', // 52 /Rho
    '\u{03A3}', // 53 /Sigma
    '\u{03A4}', // 54 /Tau
    '\u{03A5}', // 55 /Upsilon
    '\u{03C2}', // 56 /sigma1
    '\u{2126}', // 57 /Omega
    '\u{039E}', // 58 /Xi
    '\u{03A8}', // 59 /Psi
    '\u{0396}', // 5A /Zeta
    '\u{005B}', // 5B /bracketleft
    '\u{2234}', // 5C /therefore
    '\u{005D}', // 5D /bracketright
    '\u{22A5}', // 5E /perpendicular
    '\u{005F}', // 5F /underscore
    '\u{F8E5}', // 60 /radicalex
    '\u{03B1}', // 61 /alpha
    '\u{03B2}', // 62 /beta
    '\u{03C7}', // 63 /chi
    '\u{03B4}', // 64 /delta
    '\u{03B5}', // 65 /epsilon
    '\u{03C6}', // 66 /phi
    '\u{03B3}', // 67 /gamma
    '\u{03B7}', // 68 /eta
    '\u{03B9}', // 69 /iota
    '\u{03D5}', // 6A /phi1
    '\u{03BA}', // 6B /kappa
    '\u{03BB}', // 6C /lambda
    '\u{00B5}', // 6D /mu
    '\u{03BD}', // 6E /nu
    '\u{03BF}', // 6F /omicron
    '\u{03C0}', // 70 /pi
    '\u{03B8}', // 71 /theta
    '\u{03C1}', // 72 /rho
    '\u{03C3}', // 73 /sigma
    '\u{03C4}', // 74 /tau
    '\u{03C5}', // 75 /upsilon
    '\u{03D6}', // 76 /omega1
    '\u{03C9}', // 77 /omega
    '\u{03BE}', // 78 /xi
    '\u{03C8}', // 79 /psi
    '\u{03B6}', // 7A /zeta
    '\u{007B}', // 7B /braceleft
    '\u{007C}', // 7C /bar
    '\u{007D}', // 7D /braceright
    '\u{223C}', // 7E /similar
    '\u{FFFD}', // 7F /-
    '\u{FFFD}', // 80 /-
    '\u{FFFD}', // 81 /-
    '\u{FFFD}', // 82 /-
    '\u{FFFD}', // 83 /-
    '\u{FFFD}', // 84 /-
    '\u{FFFD}', // 85 /-
    '\u{FFFD}', // 86 /-
    '\u{FFFD}', // 87 /-
    '\u{FFFD}', // 88 /-
    '\u{FFFD}', // 89 /-
    '\u{FFFD}', // 8A /-
    '\u{FFFD}', // 8B /-
    '\u{FFFD}', // 8C /-
    '\u{FFFD}', // 8D /-
    '\u{FFFD}', // 8E /-
    '\u{FFFD}', // 8F /-
    '\u{FFFD}', // 90 /-
    '\u{FFFD}', // 91 /-
    '\u{FFFD}', // 92 /-
    '\u{FFFD}', // 93 /-
    '\u{FFFD}', // 94 /-
    '\u{FFFD}', // 95 /-
    '\u{FFFD}', // 96 /-
    '\u{FFFD}', // 97 /-
    '\u{FFFD}', // 98 /-
    '\u{FFFD}', // 99 /-
    '\u{FFFD}', // 9A /-
    '\u{FFFD}', // 9B /-
    '\u{FFFD}', // 9C /-
    '\u{FFFD}', // 9D /-
    '\u{FFFD}', // 9E /-
    '\u{FFFD}', // 9F /-
    '\u{20AC}', // A0 /Euro
    '\u{03D2}', // A1 /Upsilon1
    '\u{2032}', // A2 /minute
    '\u{2264}', // A3 /lessequal
    '\u{2044}', // A4 /fraction
    '\u{221E}', // A5 /infinity
    '\u{0192}', // A6 /florin
    '\u{2663}', // A7 /club
    '\u{2666}', // A8 /diamond
    '\u{2665}', // A9 /heart
    '\u{2660}', // AA /spade
    '\u{2194}', // AB /arrowboth
    '\u{2190}', // AC /arrowleft
    '\u{2191}', // AD /arrowup
    '\u{2192}', // AE /arrowright
    '\u{2193}', // AF /arrowdown
    '\u{00B0}', // B0 /degree
    '\u{00B1}', // B1 /plusminus
    '\u{2033}', // B2 /second
    '\u{2265}', // B3 /greaterequal
    '\u{00D7}', // B4 /multiply
    '\u{221D}', // B5 /proportional
    '\u{2202}', // B6 /partialdiff
    '\u{2022}', // B7 /bullet
    '\u{00F7}', // B8 /divide
    '\u{2260}', // B9 /notequal
    '\u{2261}', // BA /equivalence
    '\u{2248}', // BB /approxequal
    '\u{2026}', // BC /ellipsis
    '\u{F8E6}', // BD /arrowvertex
    '\u{F8E7}', // BE /arrowhorizex
    '\u{21B5}', // BF /carriagereturn
    '\u{2135}', // C0 /aleph
    '\u{2111}', // C1 /Ifraktur
    '\u{211C}', // C2 /Rfraktur
    '\u{2118}', // C3 /weierstrass
    '\u{2297}', // C4 /circlemultiply
    '\u{2295}', // C5 /circleplus
    '\u{2205}', // C6 /emptyset
    '\u{2229}', // C7 /intersection
    '\u{222A}', // C8 /union
    '\u{2283}', // C9 /propersuperset
    '\u{2287}', // CA /reflexsuperset
    '\u{2284}', // CB /notsubset
    '\u{2282}', // CC /propersubset
    '\u{2286}', // CD /reflexsubset
    '\u{2208}', // CE /element
    '\u{2209}', // CF /notelement
    '\u{2220}', // D0 /angle
    '\u{2207}', // D1 /gradient
    '\u{F6DA}', // D2 /registerserif
    '\u{F6D9}', // D3 /copyrightserif
    '\u{F6DB}', // D4 /trademarkserif
    '\u{220F}', // D5 /product
    '\u{221A}', // D6 /radical
    '\u{22C5}', // D7 /dotmath
    '\u{00AC}', // D8 /logicalnot
    '\u{2227}', // D9 /logicaland
    '\u{2228}', // DA /logicalor
    '\u{21D4}', // DB /arrowdblboth
    '\u{21D0}', // DC /arrowdblleft
    '\u{21D1}', // DD /arrowdblup
    '\u{21D2}', // DE /arrowdblright
    '\u{21D3}', // DF /arrowdbldown
    '\u{25CA}', // E0 /lozenge
    '\u{2329}', // E1 /angleleft
    '\u{F8E8}', // E2 /registersans
    '\u{F8E9}', // E3 /copyrightsans
    '\u{F8EA}', // E4 /trademarksans
    '\u{2211}', // E5 /summation
    '\u{F8EB}', // E6 /parenlefttp
    '\u{F8EC}', // E7 /parenleftex
    '\u{F8ED}', // E8 /parenleftbt
    '\u{F8EE}', // E9 /bracketlefttp
    '\u{F8EF}', // EA /bracketleftex
    '\u{F8F0}', // EB /bracketleftbt
    '\u{F8F1}', // EC /bracelefttp
    '\u{F8F2}', // ED /braceleftmid
    '\u{F8F3}', // EE /braceleftbt
    '\u{F8F4}', // EF /braceex
    '\u{FFFD}', // F0 /-
    '\u{232A}', // F1 /angleright
    '\u{222B}', // F2 /integral
    '\u{2320}', // F3 /integraltp
    '\u{F8F5}', // F4 /integralex
    '\u{2321}', // F5 /integralbt
    '\u{F8F6}', // F6 /parenrighttp
    '\u{F8F7}', // F7 /parenrightex
    '\u{F8F8}', // F8 /parenrightbt
    '\u{F8F9}', // F9 /bracketrighttp
    '\u{F8FA}', // FA /bracketrightex
    '\u{F8FB}', // FB /bracketrightbt
    '\u{F8FC}', // FC /bracerighttp
    '\u{F8FD}', // FD /bracerightmid
    '\u{F8FE}', // FE /bracerightbt
    '\u{FFFD}', // FF /-
];

pub(super) const ZAPF_DINGBATS: [char; 256] = [
    '\u{FFFD}', // 00 /-
    '\u{FFFD}', // 01 /-
    '\u{FFFD}', // 02 /-
    '\u{FFFD}', // 03 /-
    '\u{FFFD}', // 04 /-
    '\u{FFFD}', // 05 /-
    '\u{FFFD}', // 06 /-
    '\u{FFFD}', // 07 /-
    '\u{FFFD}', // 08 /-
    '\u{FFFD}', // 09 /-
    '\u{FFFD}', // 0A /-
    '\u{FFFD}', // 0B /-
    '\u{FFFD}', // 0C /-
    '\u{FFFD}', // 0D /-
    '\u{FFFD}', // 0E /-
    '\u{FFFD}', // 0F /-
    '\u{FFFD}', // 10 /-
    '\u{FFFD}', // 11 /-
    '\u{FFFD}', // 12 /-
    '\u{FFFD}', // 13 /-
    '\u{FFFD}', // 14 /-
    '\u{FFFD}', // 15 /-
    '\u{FFFD}', // 16 /-
    '\u{FFFD}', // 17 /-
    '\u{FFFD}', // 18 /-
    '\u{FFFD}', // 19 /-
    '\u{FFFD}', // 1A /-
    '\u{FFFD}', // 1B /-
    '\u{FFFD}', // 1C /-
    '\u{FFFD}', // 1D /-
    '\u{FFFD}', // 1E /-
    '\u{FFFD}', // 1F /-
    '\u{0020}', // 20 /space
    '\u{2701}', // 21 /a1
    '\u{2702}', // 22 /a2
    '\u{2703}', // 23 /a202
    '\u{2704}', // 24 /a3
    '\u{260E}', // 25 /a4
    '\u{2706}', // 26 /a5
    '\u{2707}', // 27 /a119
    '\u{2708}', // 28 /a118
    '\u{2709}', // 29 /a117
    '\u{261B}', // 2A /a11
    '\u{261E}', // 2B /a12
    '\u{270C}', // 2C /a13
    '\u{270D}', // 2D /a14
    '\u{270E}', // 2E /a15
    '\u{270F}', // 2F /a16
    '\u{2710}', // 30 /a105
    '\u{2711}', // 31 /a17
    '\u{2712}', // 32 /a18
    '\u{2713}', // 33 /a19
    '\u{2714}', // 34 /a20
    '\u{2715}', // 35 /a21
    '\u{2716}', // 36 /a22
    '\u{2717}', // 37 /a23
    '\u{2718}', // 38 /a24
    '\u{2719}', // 39 /a25
    '\u{271A}', // 3A /a26
    '\u{271B}', // 3B /a27
    '\u{271C}', // 3C /a28
    '\u{271D}', // 3D /a6
    '\u{271E}', // 3E /a7
    '\u{271F}', // 3F /a8
    '\u{2720}', // 40 /a9
    '\u{2721}', // 41 /a10
    '\u{2722}', // 42 /a29
    '\u{2723}', // 43 /a30
    '\u{2724}', // 44 /a31
    '\u{2725}', // 45 /a32
    '\u{2726}', // 46 /a33
    '\u{2727}', // 47 /a34
    '\u{2605}', // 48 /a35
    '\u{2729}', // 49 /a36
    '\u{272A}', // 4A /a37
    '\u{272B}', // 4B /a38
    '\u{272C}', // 4C /a39
    '\u{272D}', // 4D /a40
    '\u{272E}', // 4E /a41
    '\u{272F}', // 4F /a42
    '\u{2730}', // 50 /a43
    '\u{2731}', // 51 /a44
    '\u{2732}', // 52 /a45
    '\u{2733}', // 53 /a46
    '\u{2734}', // 54 /a47
    '\u{2735}', // 55 /a48
    '\u{2736}', // 56 /a49
    '\u{2737}', // 57 /a50
    '\u{2738}', // 58 /a51
    '\u{2739}', // 59 /a52
    '\u{273A}', // 5A /a53
    '\u{273B}', // 5B /a54
    '\u{273C}', // 5C /a55
    '\u{273D}', // 5D /a56
    '\u{273E}', // 5E /a57
    '\u{273F}', // 5F /a58
    '\u{2740}', // 60 /a59
    '\u{2741}', // 61 /a60
    '\u{2742}', // 62 /a61
    '\u{2743}', // 63 /a62
    '\u{2744}', // 64 /a63
    '\u{2745}', // 65 /a64
    '\u{2746}', // 66 /a65
    '\u{2747}', // 67 /a66
    '\u{2748}', // 68 /a67
    '\u{2749}', // 69 /a68
    '\u{274A}', // 6A /a69
    '\u{274B}', // 6B /a70
    '\u{25CF}', // 6C /a71
    '\u{274D}', // 6D /a72
    '\u{25A0}', // 6E /a73
    '\u{274F}', // 6F /a74
    '\u{2750}', // 70 /a203
    '\u{2751}', // 71 /a75
    '\u{2752}', // 72 /a204
    '\u{25B2}', // 73 /a76
    '\u{25BC}', // 74 /a77
    '\u{25C6}', // 75 /a78
    '\u{2756}', // 76 /a79
    '\u{25D7}', // 77 /a81
    '\u{2758}', // 78 /a82
    '\u{2759}', // 79 /a83
    '\u{275A}', // 7A /a84
    '\u{275B}', // 7B /a97
    '\u{275C}', // 7C /a98
    '\u{275D}', // 7D /a99
    '\u{275E}', // 7E /a100
    '\u{FFFD}', // 7F /-
    '\u{FFFD}', // 80 /-
    '\u{FFFD}', // 81 /-
    '\u{FFFD}', // 82 /-
    '\u{FFFD}', // 83 /-
    '\u{FFFD}', // 84 /-
    '\u{FFFD}', // 85 /-
    '\u{FFFD}', // 86 /-
    '\u{FFFD}', // 87 /-
    '\u{FFFD}', // 88 /-
    '\u{FFFD}', // 89 /-
    '\u{FFFD}', // 8A /-
    '\u{FFFD}', // 8B /-
    '\u{FFFD}', // 8C /-
    '\u{FFFD}', // 8D /-
    '\u{FFFD}', // 8E /-
    '\u{FFFD}', // 8F /-
    '\u{FFFD}', // 90 /-
    '\u{FFFD}', // 91 /-
    '\u{FFFD}', // 92 /-
    '\u{FFFD}', // 93 /-
    '\u{FFFD}', // 94 /-
    '\u{FFFD}', // 95 /-
    '\u{FFFD}', // 96 /-
    '\u{FFFD}', // 97 /-
    '\u{FFFD}', // 98 /-
    '\u{FFFD}', // 99 /-
    '\u{FFFD}', // 9A /-
    '\u{FFFD}', // 9B /-
    '\u{FFFD}', // 9C /-
    '\u{FFFD}', // 9D /-
    '\u{FFFD}', // 9E /-
    '\u{FFFD}', // 9F /-
    '\u{FFFD}', // A0 /-
    '\u{2761}', // A1 /a101
    '\u{2762}', // A2 /a102
    '\u{2763}', // A3 /a103
    '\u{2764}', // A4 /a104
    '\u{2765}', // A5 /a106
    '\u{2766}', // A6 /a107
    '\u{2767}', // A7 /a108
    '\u{2663}', // A8 /a112
    '\u{2666}', // A9 /a111
    '\u{2665}', // AA /a110
    '\u{2660}', // AB /a109
    '\u{2460}', // AC /a120
    '\u{2461}', // AD /a121
    '\u{2462}', // AE /a122
    '\u{2463}', // AF /a123
    '\u{2464}', // B0 /a124
    '\u{2465}', // B1 /a125
    '\u{2466}', // B2 /a126
    '\u{2467}', // B3 /a127
    '\u{2468}', // B4 /a128
    '\u{2469}', // B5 /a129
    '\u{2776}', // B6 /a130
    '\u{2777}', // B7 /a131
    '\u{2778}', // B8 /a132
    '\u{2779}', // B9 /a133
    '\u{277A}', // BA /a134
    '\u{277B}', // BB /a135
    '\u{277C}', // BC /a136
    '\u{277D}', // BD /a137
    '\u{277E}', // BE /a138
    '\u{277F}', // BF /a139
    '\u{2780}', // C0 /a140
    '\u{2781}', // C1 /a141
    '\u{2782}', // C2 /a142
    '\u{2783}', // C3 /a143
    '\u{2784}', // C4 /a144
    '\u{2785}', // C5 /a145
    '\u{2786}', // C6 /a146
    '\u{2787}', // C7 /a147
    '\u{2788}', // C8 /a148
    '\u{2789}', // C9 /a149
    '\u{278A}', // CA /a150
    '\u{278B}', // CB /a151
    '\u{278C}', // CC /a152
    '\u{278D}', // CD /a153
    '\u{278E}', // CE /a154
    '\u{278F}', // CF /a155
    '\u{2790}', // D0 /a156
    '\u{2791}', // D1 /a157
    '\u{2792}', // D2 /a158
    '\u{2793}', // D3 /a159
    '\u{2794}', // D4 /a160
    '\u{2192}', // D5 /a161
    '\u{2194}', // D6 /a163
    '\u{2195}', // D7 /a164
    '\u{2798}', // D8 /a196
    '\u{2799}', // D9 /a165
    '\u{279A}', // DA /a192
    '\u{279B}', // DB /a166
    '\u{279C}', // DC /a167
    '\u{279D}', // DD /a168
    '\u{279E}', // DE /a169
    '\u{279F}', // DF /a170
    '\u{27A0}', // E0 /a171
    '\u{27A1}', // E1 /a172
    '\u{27A2}', // E2 /a173
    '\u{27A3}', // E3 /a162
    '\u{27A4}', // E4 /a174
    '\u{27A5}', // E5 /a175
    '\u{27A6}', // E6 /a176
    '\u{27A7}', // E7 /a177
    '\u{27A8}', // E8 /a178
    '\u{27A9}', // E9 /a179
    '\u{27AA}', // EA /a193
    '\u{27AB}', // EB /a180
    '\u{27AC}', // EC /a199
    '\u{27AD}', // ED /a181
    '\u{27AE}', // EE /a200
    '\u{27AF}', // EF /a182
    '\u{FFFD}', // F0 /-
    '\u{27B1}', // F1 /a201
    '\u{27B2}', // F2 /a183
    '\u{27B3}', // F3 /a184
    '\u{27B4}', // F4 /a197
    '\u{27B5}', // F5 /a185
    '\u{27B6}', // F6 /a194
    '\u{27B7}', // F7 /a198
    '\u{27B8}', // F8 /a186
    '\u{27B9}', // F9 /a195
    '\u{27BA}', // FA /a187
    '\u{27BB}', // FB /a188
    '\u{27BC}', // FC /a189
    '\u{27BD}', // FD /a190
    '\u{27BE}', // FE /a191
    '\u{FFFD}', // FF /-
];

// Names are sorted for bounded binary lookup; these are specific to Zapf fonts.
const ZAPF_GLYPHS: &[(&str, char)] = &[
    ("a1", '\u{2701}'),
    ("a10", '\u{2721}'),
    ("a100", '\u{275E}'),
    ("a101", '\u{2761}'),
    ("a102", '\u{2762}'),
    ("a103", '\u{2763}'),
    ("a104", '\u{2764}'),
    ("a105", '\u{2710}'),
    ("a106", '\u{2765}'),
    ("a107", '\u{2766}'),
    ("a108", '\u{2767}'),
    ("a109", '\u{2660}'),
    ("a11", '\u{261B}'),
    ("a110", '\u{2665}'),
    ("a111", '\u{2666}'),
    ("a112", '\u{2663}'),
    ("a117", '\u{2709}'),
    ("a118", '\u{2708}'),
    ("a119", '\u{2707}'),
    ("a12", '\u{261E}'),
    ("a120", '\u{2460}'),
    ("a121", '\u{2461}'),
    ("a122", '\u{2462}'),
    ("a123", '\u{2463}'),
    ("a124", '\u{2464}'),
    ("a125", '\u{2465}'),
    ("a126", '\u{2466}'),
    ("a127", '\u{2467}'),
    ("a128", '\u{2468}'),
    ("a129", '\u{2469}'),
    ("a13", '\u{270C}'),
    ("a130", '\u{2776}'),
    ("a131", '\u{2777}'),
    ("a132", '\u{2778}'),
    ("a133", '\u{2779}'),
    ("a134", '\u{277A}'),
    ("a135", '\u{277B}'),
    ("a136", '\u{277C}'),
    ("a137", '\u{277D}'),
    ("a138", '\u{277E}'),
    ("a139", '\u{277F}'),
    ("a14", '\u{270D}'),
    ("a140", '\u{2780}'),
    ("a141", '\u{2781}'),
    ("a142", '\u{2782}'),
    ("a143", '\u{2783}'),
    ("a144", '\u{2784}'),
    ("a145", '\u{2785}'),
    ("a146", '\u{2786}'),
    ("a147", '\u{2787}'),
    ("a148", '\u{2788}'),
    ("a149", '\u{2789}'),
    ("a15", '\u{270E}'),
    ("a150", '\u{278A}'),
    ("a151", '\u{278B}'),
    ("a152", '\u{278C}'),
    ("a153", '\u{278D}'),
    ("a154", '\u{278E}'),
    ("a155", '\u{278F}'),
    ("a156", '\u{2790}'),
    ("a157", '\u{2791}'),
    ("a158", '\u{2792}'),
    ("a159", '\u{2793}'),
    ("a16", '\u{270F}'),
    ("a160", '\u{2794}'),
    ("a161", '\u{2192}'),
    ("a162", '\u{27A3}'),
    ("a163", '\u{2194}'),
    ("a164", '\u{2195}'),
    ("a165", '\u{2799}'),
    ("a166", '\u{279B}'),
    ("a167", '\u{279C}'),
    ("a168", '\u{279D}'),
    ("a169", '\u{279E}'),
    ("a17", '\u{2711}'),
    ("a170", '\u{279F}'),
    ("a171", '\u{27A0}'),
    ("a172", '\u{27A1}'),
    ("a173", '\u{27A2}'),
    ("a174", '\u{27A4}'),
    ("a175", '\u{27A5}'),
    ("a176", '\u{27A6}'),
    ("a177", '\u{27A7}'),
    ("a178", '\u{27A8}'),
    ("a179", '\u{27A9}'),
    ("a18", '\u{2712}'),
    ("a180", '\u{27AB}'),
    ("a181", '\u{27AD}'),
    ("a182", '\u{27AF}'),
    ("a183", '\u{27B2}'),
    ("a184", '\u{27B3}'),
    ("a185", '\u{27B5}'),
    ("a186", '\u{27B8}'),
    ("a187", '\u{27BA}'),
    ("a188", '\u{27BB}'),
    ("a189", '\u{27BC}'),
    ("a19", '\u{2713}'),
    ("a190", '\u{27BD}'),
    ("a191", '\u{27BE}'),
    ("a192", '\u{279A}'),
    ("a193", '\u{27AA}'),
    ("a194", '\u{27B6}'),
    ("a195", '\u{27B9}'),
    ("a196", '\u{2798}'),
    ("a197", '\u{27B4}'),
    ("a198", '\u{27B7}'),
    ("a199", '\u{27AC}'),
    ("a2", '\u{2702}'),
    ("a20", '\u{2714}'),
    ("a200", '\u{27AE}'),
    ("a201", '\u{27B1}'),
    ("a202", '\u{2703}'),
    ("a203", '\u{2750}'),
    ("a204", '\u{2752}'),
    ("a21", '\u{2715}'),
    ("a22", '\u{2716}'),
    ("a23", '\u{2717}'),
    ("a24", '\u{2718}'),
    ("a25", '\u{2719}'),
    ("a26", '\u{271A}'),
    ("a27", '\u{271B}'),
    ("a28", '\u{271C}'),
    ("a29", '\u{2722}'),
    ("a3", '\u{2704}'),
    ("a30", '\u{2723}'),
    ("a31", '\u{2724}'),
    ("a32", '\u{2725}'),
    ("a33", '\u{2726}'),
    ("a34", '\u{2727}'),
    ("a35", '\u{2605}'),
    ("a36", '\u{2729}'),
    ("a37", '\u{272A}'),
    ("a38", '\u{272B}'),
    ("a39", '\u{272C}'),
    ("a4", '\u{260E}'),
    ("a40", '\u{272D}'),
    ("a41", '\u{272E}'),
    ("a42", '\u{272F}'),
    ("a43", '\u{2730}'),
    ("a44", '\u{2731}'),
    ("a45", '\u{2732}'),
    ("a46", '\u{2733}'),
    ("a47", '\u{2734}'),
    ("a48", '\u{2735}'),
    ("a49", '\u{2736}'),
    ("a5", '\u{2706}'),
    ("a50", '\u{2737}'),
    ("a51", '\u{2738}'),
    ("a52", '\u{2739}'),
    ("a53", '\u{273A}'),
    ("a54", '\u{273B}'),
    ("a55", '\u{273C}'),
    ("a56", '\u{273D}'),
    ("a57", '\u{273E}'),
    ("a58", '\u{273F}'),
    ("a59", '\u{2740}'),
    ("a6", '\u{271D}'),
    ("a60", '\u{2741}'),
    ("a61", '\u{2742}'),
    ("a62", '\u{2743}'),
    ("a63", '\u{2744}'),
    ("a64", '\u{2745}'),
    ("a65", '\u{2746}'),
    ("a66", '\u{2747}'),
    ("a67", '\u{2748}'),
    ("a68", '\u{2749}'),
    ("a69", '\u{274A}'),
    ("a7", '\u{271E}'),
    ("a70", '\u{274B}'),
    ("a71", '\u{25CF}'),
    ("a72", '\u{274D}'),
    ("a73", '\u{25A0}'),
    ("a74", '\u{274F}'),
    ("a75", '\u{2751}'),
    ("a76", '\u{25B2}'),
    ("a77", '\u{25BC}'),
    ("a78", '\u{25C6}'),
    ("a79", '\u{2756}'),
    ("a8", '\u{271F}'),
    ("a81", '\u{25D7}'),
    ("a82", '\u{2758}'),
    ("a83", '\u{2759}'),
    ("a84", '\u{275A}'),
    ("a9", '\u{2720}'),
    ("a97", '\u{275B}'),
    ("a98", '\u{275C}'),
    ("a99", '\u{275D}'),
    ("space", '\u{0020}'),
];

pub(super) fn zapf_glyph(name: &str) -> Option<char> {
    let base = name.split_once('.').map_or(name, |(base, _)| base);
    ZAPF_GLYPHS
        .binary_search_by_key(&base, |(name, _)| *name)
        .ok()
        .map(|index| ZAPF_GLYPHS[index].1)
}
