#include <stdio.h>

extern double add(double, double);
extern double sub(double, double);
extern double mul(double, double);
extern double div(double, double);

int main() {
    printf("%.2f", mul(150, add(10, 20)));
}
