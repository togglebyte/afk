
Usage: afk "some text to show" -h # -m # -s # -k -c blue

Text to display can be empty, a single word, or a "quoted string" of words.

-h #  Number of hours to count down
-m #  Number of minutes to count down
-s #  Number of seconds to count down
You can enter time in any combination of hms or just one.
The application will adjust it. Ex: -s 90 will translate to 1m 30s.
Color can be a comma or quoted space separated RGB value: 42,42,42 or "42 42 42"

-c color  colors the text with a bold foreground color.
Colors: Black, Red, Green, Yellow, Blue, Purple, Cyan, White

-k  Allow countdown to go negative / Stopwatch mode

-0 Hide hour or minutes when zero

-f Use figgle font for message

--help  shows this help
