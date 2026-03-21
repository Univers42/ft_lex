# ft_lex Makefile
NAME        = ft_lex
LIBL        = libl.a
CC          = cc
CFLAGS      = -Wall -Wextra -Werror
AR          = ar rcs
LIBL_SRC    = libl.c
LIBL_OBJ    = libl.o
BIN         = ./bin/ft_lex

all: $(BIN) $(LIBL)

$(BIN):
	cargo build --release -p ft_lex
	mkdir -p ./bin
	cp target/release/ft_lex $(BIN)

$(LIBL): $(LIBL_OBJ)
	$(AR) $(LIBL) $(LIBL_OBJ)

$(LIBL_OBJ): $(LIBL_SRC)
	$(CC) $(CFLAGS) -c $(LIBL_SRC) -o $(LIBL_OBJ)

clean:
	cargo clean
	rm -f $(LIBL_OBJ)

fclean: clean
	rm -rf ./bin $(LIBL)

re: fclean all

.PHONY: all clean fclean re
