# Sources

The documents are not kept in the repository. Fetch them from the addresses
below into the directories named, if you want to check a reading.

The documents this implementation was written from, as downloaded on
22 November 2026.

| Directory | Document | From |
| --- | --- | --- |
| `working-papers-1966/` | *CPL Working Papers*, July 1966 (128 pages, scanned). The Reference Manual draft, the preprocessor program (Appendix 1), the switch-command, the list of changes foreseen. | https://www.ancientgeek.org.uk/CPL/CPL_Working_Papers.pdf |
| `elementary-manual-1965/` | *CPL Elementary Programming Manual* (46 pages, scanned), with the 1966 revision pages and the pre-pilot system supplement. | https://www.ancientgeek.org.uk/CPL/CPL_Elementary_Programming_Manual.pdf |
| `main-features-1963/` | Barron, Buxton, Hartley, Nixon and Strachey, "The main features of CPL", *Computer Journal* 6, 1963. | https://www.ancientgeek.org.uk/CPL/The_Main_Features_of_CPL.pdf |
| `richards-cpl2bcpl/` | Martin Richards, "How BCPL evolved from CPL". | https://www.cl.cam.ac.uk/~mr10/cpl2bcpl.pdf |
| `london-cpl1-compiler/` | Coulouris, Goodey, Hill, Keeling and Levin, "The London CPL1 compiler" (scanned). Background only. | https://www.ancientgeek.org.uk/CPL/The_London_CPL1_Compiler.pdf |

The `.txt` files were extracted with `pdftotext -layout`. The `.ocr.txt`
files are the scanned documents put through Tesseract a page at a time
(`pdftoppm -r 250 -gray`, then `tesseract --psm 4`), with a line of `=`
signs and the page number between pages.

The OCR is poor wherever the original uses underlining, subscripts or the
special characters of the CPL alphabet, which is to say in every program and
every syntax table. It is good enough to find a passage; the passage should
then be read in the PDF. One example of what it does: the routine of
section 9.3.2, which on page 92 of the PDF is

    let recursive routine R[integer n, label L, M] be
    § if n > 0 do R[n-1, M, N]
      N: goto L

comes out as `R \jinteger n, label L, u| be § if n>OdoR [n-1, M, n]`.

The Elementary Programming Manual was also offered at
chilton-computing.org.uk, which did not resolve when these were fetched.
