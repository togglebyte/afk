**Note** this entire project is a bit of a hack thrown together as I needed a
timer.

Usage: afk "some text to show" -h # -m # -s # -k -c blue

Text to display can be empty, a single word, or a "quoted string" of words.

-h #  Number of hours to count down
-m #  Number of minutes to count down
-s #  Number of seconds to count down
You can enter time in any combination of hms or just one.
The application will adjust it. Ex: -s 90 will translate to 1m 30s.
Color can be an comma separated RGB value: 42,42,42

-c color  colors the text with a bold foreground color.
Colors: Black, Red, Green, Yellow, Blue, Purple, Cyan, White

-k  Allow countdown to go negative / Stopwatch mode

--help  shows this help
