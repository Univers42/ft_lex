%{
#include <stdio.h>
%}
%%
"if"        printf("KW_IF\n");
"else"      printf("KW_ELSE\n");
"while"     printf("KW_WHILE\n");
"return"    printf("KW_RETURN\n");
[a-zA-Z_][a-zA-Z0-9_]* printf("ID: %s\n", yytext);
[0-9]+                  printf("NUM: %s\n", yytext);
[ \t\n]                 ;
.                       ;
%%
int yywrap(void) { return 1; }
int main(void) { while (yylex() != 0); return 0; }
