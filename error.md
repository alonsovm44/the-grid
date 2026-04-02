   Compiling the-grid v0.1.0 (C:\Users\diego\.projects\TheGrid)
error: unknown character escape: ` `
   --> src\agent.rs:680:67
    |
680 |                     - Use \"speak\" for general announcements or \ uubs
    |                                                                   ^ unknown character escape
    |
    = help: for more information, visit <https://doc.rust-lang.org/reference/tokens.html#literals>
help: if you meant to write a literal backslash (perhaps escaping in a regular expression), consider a raw string literal
    |
645 |                     r"You are {name}, an autonomous program living inside a system called The Grid.
    |                     +

error: unknown start of token: \
   --> src\agent.rs:786:40
    |
786 |             .collect::<Vec<_>>().join("\n");
    |                                        ^

error: prefix `n` is unknown
   --> src\agent.rs:786:41
    |
786 |             .collect::<Vec<_>>().join("\n");
    |                                         ^ unknown prefix
    |
    = note: prefixed identifiers and literals are reserved since Rust 2021
help: consider inserting whitespace here
    |
786 |             .collect::<Vec<_>>().join("\n ");
    |                                          +

error: unknown start of token: \
   --> src\agent.rs:789:35
    |
789 |             "SPECIALIZATION RULES:\n\
    |                                   ^

error: unknown start of token: \
   --> src\agent.rs:789:37
    |
789 |             "SPECIALIZATION RULES:\n\
    |                                     ^

error: character literal may only contain one codepoint
   --> src\agent.rs:790:33
    |
790 | ...   - You represent the '{}' tool. When using \"execute_command\", you must ONLY run commands specific to your domain (e.g., if you are 'git', run 'git' commands).\n\ 
    |                           ^^^^
    |
help: if you meant to write a string literal, use double quotes
    |
790 -             - You represent the '{}' tool. When using \"execute_command\", you must ONLY run commands specific to your domain (e.g., if you are 'git', run 'git' commands).\n\
790 +             - You represent the "{}" tool. When using \"execute_command\", you must ONLY run commands specific to your domain (e.g., if you are 'git', run 'git' commands).\n\
    |

error: unknown start of token: \
   --> src\agent.rs:790:55
    |
790 | ...   - You represent the '{}' tool. When using \"execute_command\", you must ONLY run commands specific to your domain (e.g., if you are 'git', run 'git' commands).\n\ 
    |                                                 ^

error: unknown character escape: `f`
   --> src\agent.rs:820:48
    |
820 |             Current directory: {current_dir}\n\f
    |                                                ^ unknown character escape
    |
    = help: for more information, visit <https://doc.rust-lang.org/reference/tokens.html#literals>
help: if you meant to write a literal backslash (perhaps escaping in a regular expression), consider a raw string literal
    |
798 |             r"You are {name}, an autonomous program on The Grid.\n\
    |             +

error: unknown character escape: `a`
   --> src\agent.rs:825:28
    |
825 |             JSON FORMAT:\n\ate_task\" | \"write_file\" | \"read_dir\" | \"create_dir\" | \"complete_task\" | \"read_web\",\n\
    |                            ^ unknown character escape
    |
    = help: for more information, visit <https://doc.rust-lang.org/reference/tokens.html#literals>
help: if you meant to write a literal backslash (perhaps escaping in a regular expression), consider a raw string literal
    |
798 |             r"You are {name}, an autonomous program on The Grid.\n\
    |             +

error: unknown character escape: ` `
   --> src\agent.rs:921:43
    |
921 |             Readable files: {file_list}\n\
    |                                           ^ unknown character escape
    |
    = help: for more information, visit <https://doc.rust-lang.org/reference/tokens.html#literals>
help: if you meant to write a literal backslash (perhaps escaping in a regular expression), consider a raw string literal
    |
896 |             r"You are {name}, an autonomous program on The Grid.\n\
    |             +

error: unknown character escape: ` `
   --> src\agent.rs:931:59
    |
931 |             \"url\": \"string (required for read_web)\"\n\
    |                                                           ^ unknown character escape
    |
    = help: for more information, visit <https://doc.rust-lang.org/reference/tokens.html#literals>
help: if you meant to write a literal backslash (perhaps escaping in a regular expression), consider a raw string literal
    |
896 |             r"You are {name}, an autonomous program on The Grid.\n\
    |             +

error: prefix `restricted` is unknown
    --> src\agent.rs:1017:125
     |
1017 |                 let _ = self.tx.send(Event { sender: self.name.clone(), action: "feels".to_string(), content: "silenced and restricted".to_string() });
     |                                                                                                                             ^^^^^^^^^^ unknown prefix
     |
     = note: prefixed identifiers and literals are reserved since Rust 2021
help: consider inserting whitespace here
     |
1017 |                 let _ = self.tx.send(Event { sender: self.name.clone(), action: "feels".to_string(), content: "silenced and restricted ".to_string() });
     |                                                                                                                                       +

error: prefix `restored` is unknown
    --> src\agent.rs:1022:130
     |
1022 |                 let _ = self.tx.send(Event { sender: self.name.clone(), action: "feels".to_string(), content: "vocal subroutines restored".to_string() });
     |                                                                                                                                  ^^^^^^^^ unknown prefix
     |
     = note: prefixed identifiers and literals are reserved since Rust 2021
help: consider inserting whitespace here
     |
1022 |                 let _ = self.tx.send(Event { sender: self.name.clone(), action: "feels".to_string(), content: "vocal subroutines restored ".to_string() });
     |                                                                                                                                          +

error: character literal may only contain one codepoint
    --> src\agent.rs:1047:101
     |
1047 | ...                   "Your personality: {}. You are a program named {}. Your current mood is '{}'.\n\
     |                                                                                               ^^^^
     |
help: if you meant to write a string literal, use double quotes
     |
1047 -                             "Your personality: {}. You are a program named {}. Your current mood is '{}'.\n\
1047 +                             "Your personality: {}. You are a program named {}. Your current mood is "{}".\n\
     |

error: unknown start of token: \
    --> src\agent.rs:1047:106
     |
1047 | ...                   "Your personality: {}. You are a program named {}. Your current mood is '{}'.\n\
     |                                                                                                    ^

error: unknown start of token: \
    --> src\agent.rs:1047:108
     |
1047 | ...                   "Your personality: {}. You are a program named {}. Your current mood is '{}'.\n\
     |                                                                                                      ^

error: character literal may only contain one codepoint
    --> src\agent.rs:1048:51
     |
1048 | ...                   SYSTEM ALERT: Program '{}' has just been {} by the User.\n\
     |                                             ^^^^
     |
help: if you meant to write a string literal, use double quotes
     |
1048 -                             SYSTEM ALERT: Program '{}' has just been {} by the User.\n\
1048 +                             SYSTEM ALERT: Program "{}" has just been {} by the User.\n\
     |

error: unknown start of token: \
    --> src\agent.rs:1048:85
     |
1048 | ...                   SYSTEM ALERT: Program '{}' has just been {} by the User.\n\
     |                                                                               ^

error: unknown start of token: \
    --> src\agent.rs:1048:87
     |
1048 | ...                   SYSTEM ALERT: Program '{}' has just been {} by the User.\n\
     |                                                                                 ^

error: character literal may only contain one codepoint
    --> src\agent.rs:1084:58
     |
1084 |                         let full_task = format!("Program '{}' delegated this sub-task to you: {}", event.sender, task_desc);
     |                                                          ^^^^
     |
help: if you meant to write a string literal, use double quotes
     |
1084 -                         let full_task = format!("Program '{}' delegated this sub-task to you: {}", event.sender, task_desc);
1084 +                         let full_task = format!("Program "{}" delegated this sub-task to you: {}", event.sender, task_desc);
     |

error: unknown start of token: \
    --> src\agent.rs:1107:112
     |
1107 | ...                   "You are {name}, an autonomous program fighting in a Lightcycles Arena on The Grid!\n\
     |                                                                                                          ^

error: unknown start of token: \
    --> src\agent.rs:1107:114
     |
1107 | ...                   "You are {name}, an autonomous program fighting in a Lightcycles Arena on The Grid!\n\
     |                                                                                                            ^

error: unknown start of token: \
    --> src\agent.rs:1108:70
     |
1108 | ...                   Combat Experience: {xp} XP. {xp_guidance}\n\
     |                                                                ^

error: unknown start of token: \
    --> src\agent.rs:1108:72
     |
1108 | ...                   Combat Experience: {xp} XP. {xp_guidance}\n\
     |                                                                  ^

error: unknown start of token: \
    --> src\agent.rs:1109:42
     |
1109 | ...                   {board_state}\n\
     |                                    ^

error: unknown start of token: \
    --> src\agent.rs:1109:44
     |
1109 | ...                   {board_state}\n\
     |                                      ^

error: unknown start of token: \
    --> src\agent.rs:1110:29
     |
1110 | ...                   \n\
     |                       ^

error: unknown start of token: \
    --> src\agent.rs:1110:31
     |
1110 | ...                   \n\
     |                         ^

error: unknown start of token: \
    --> src\agent.rs:1111:35
     |
1111 | ...                   RULES:\n\
     |                             ^

error: unknown start of token: \
    --> src\agent.rs:1111:37
     |
1111 | ...                   RULES:\n\
     |                               ^

error: unknown start of token: \
    --> src\agent.rs:1112:109
     |
1112 | ...                   1. '.' is empty space. '#' are deadly light trails. 'A' and 'B' are the players.\n\
     |                                                                                                       ^

error: unknown start of token: \
    --> src\agent.rs:1112:111
     |
1112 | ...                   1. '.' is empty space. '#' are deadly light trails. 'A' and 'B' are the players.\n\
     |                                                                                                         ^

error: unknown start of token: \
    --> src\agent.rs:1113:107
     |
1113 | ...                   2. You must avoid crashing into walls (boundaries) and trails ('#', 'A', 'B').\n\
     |                                                                                                     ^

error: unknown start of token: \
    --> src\agent.rs:1113:109
     |
1113 | ...                   2. You must avoid crashing into walls (boundaries) and trails ('#', 'A', 'B').\n\
     |                                                                                                       ^

error: unknown start of token: \
    --> src\agent.rs:1114:98
     |
1114 | ...                   3. Your goal is to outmaneuver the other program and make them crash.\n\
     |                                                                                            ^

error: unknown start of token: \
    --> src\agent.rs:1114:100
     |
1114 | ...                   3. Your goal is to outmaneuver the other program and make them crash.\n\
     |                                                                                              ^

error: unknown start of token: \
    --> src\agent.rs:1115:112
     |
1115 | ...                   4. Output ONLY valid JSON indicating your next direction of travel. Do not explain.\n\
     |                                                                                                          ^

error: unknown start of token: \
    --> src\agent.rs:1115:114
     |
1115 | ...                   4. Output ONLY valid JSON indicating your next direction of travel. Do not explain.\n\
     |                                                                                                            ^

error: unknown start of token: \
    --> src\agent.rs:1116:29
     |
1116 | ...                   \n\
     |                       ^

error: unknown start of token: \
    --> src\agent.rs:1116:31
     |
1116 | ...                   \n\
     |                         ^

error: unknown start of token: \
    --> src\agent.rs:1117:41
     |
1117 | ...                   JSON FORMAT:\n\
     |                                   ^

error: unknown start of token: \
    --> src\agent.rs:1117:43
     |
1117 | ...                   JSON FORMAT:\n\
     |                                     ^

error: unknown start of token: \
    --> src\agent.rs:1118:31
     |
1118 | ...                   {{\n\
     |                         ^

error: unknown start of token: \
    --> src\agent.rs:1118:33
     |
1118 | ...                   {{\n\
     |                           ^

error: unknown start of token: \
    --> src\agent.rs:1119:29
     |
1119 | ...                   \"action\": \"play_move\",\n\
     |                       ^

error: mismatched closing delimiter: `}`
    --> src\agent.rs:797:29
     |
 797 |         let prompt = format!(
     |                             ^ unclosed delimiter
...
1129 |                     }
     |                     ^ mismatched closing delimiter

error: unexpected closing delimiter: `}`
    --> src\agent.rs:1131:13
     |
1118 |                             {{\n\
     |                              - the nearest open delimiter
...
1126 |                         );
     |                         - missing open `(` for this delimiter
...
1131 |             }
     |             ^ unexpected closing delimiter

error: could not compile `the-grid` (bin "the-grid") due to 47 previous errors