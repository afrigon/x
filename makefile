all: main

clean:
	rm -f main main.o lib.o

main: main.o lib.o
	clang -o main main.o lib.o

main.o: main.x
	cargo run -- compile main.x

lib.o: lib.c
	clang -o lib.o -c lib.c

