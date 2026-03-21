%option noyywrap
%%
[a-zA-Z_][a-zA-Z0-9_]* printf("ID: %s\n", yytext);
[0-9]+                  printf("NUM: %s\n", yytext);
[0-9]+"."[0-9]*         printf("FLOAT: %s\n", yytext);
"="                     printf("ASSIGN\n");
"=="                    printf("EQ\n");
"!="                    printf("NE\n");
";"                     printf("SEMI\n");
[ \t\n]                 ;
.                       printf("UNKNOWN: %s\n", yytext);
