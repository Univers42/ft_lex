/* libl.c — lex runtime library (ft_lex) */
#include <stdio.h>

extern int yylex(void);

int yywrap(void) {
    return 1;
}

int main(void) {
    while (yylex() != 0)
        ;
    return 0;
}
