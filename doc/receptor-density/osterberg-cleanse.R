# Function for automatically installing and loading of packages, so that the code runs on every R instance.
pkgLoad <- function(x) {
  if (!require(x, character.only = TRUE)) {
    chooseCRANmirror(ind = 33)
    install.packages(x, dep = TRUE)
    if (!require(x, character.only = TRUE))
      stop("Package not found")
  }
  suppressPackageStartupMessages(library(x, character.only = TRUE))
}

# Returns the capitalized string.
toCapitalized <- function(x) {
  gsub("(?<=\\b)([a-z])", "\\U\\1", tolower(x), perl = TRUE)
}

# Set working directory.
pkgLoad("rstudioapi")
setwd(dirname(getActiveDocumentContext()$path))

# Packages.
pkgLoad("ggplot2")
pkgLoad("scales")

# Load and cleanse data.
data <- read.csv(file = "osterberg-digitized.csv",header = TRUE, stringsAsFactors = FALSE)
offset = data$RodsX[which.min(data$RodsY)]
data$ConesX <- data$ConesX - offset
data$ConesY <- rescale(data$ConesY, to = c(0, 174))
data$RodsX <- data$RodsX - offset
data$RodsY <- rescale(data$RodsY, to = c(0, 162))

ggplot(data = data) +
  geom_line(aes(x = ConesX, y = ConesY), color = "red", linetype = "solid", size = 0.25, alpha = 0.5) +
  geom_line(aes(x = RodsX, y = RodsY), color = "blue", linetype = "solid", size = 0.25, alpha = 0.5) +
  scale_x_continuous(breaks=c(0))

# Extract, round, and sort.
set.seed(42)
cone_data <- na.omit(data[c("ConesX", "ConesY")])
cone_data <- cone_data[order(cone_data$ConesX),]
cone_data$ConesX <- round(cone_data$ConesX + runif(nrow(cone_data), min=0.0001, max=0.0009), 4)
cone_data$ConesY <- round(cone_data$ConesY, 2)
rod_data <- na.omit(data[c("RodsX", "RodsY")])
rod_data <- rod_data[order(rod_data$RodsX),]
rod_data$RodsX <- round(rod_data$RodsX + runif(nrow(rod_data), min=0.0001, max=0.0009), 4)
rod_data$RodsY <- round(rod_data$RodsY, 2)

any(duplicated(rod_data$RodsX))
any(duplicated(cone_data$ConesX))

write.csv(cone_data, file = "cones.csv", quote = FALSE, row.names = FALSE, na = "")
write.csv(rod_data, file = "rods.csv", quote = FALSE, row.names = FALSE, na = "")

ggplot() +
  geom_line(data = cone_data, aes(x = ConesX, y = ConesY), color = "red", linetype = "solid", size = 0.25, alpha = 0.5) +
  geom_line(data = rod_data, aes(x = RodsX, y = RodsY), color = "blue", linetype = "solid", size = 0.25, alpha = 0.5) +
  geom_point(data = cone_data, aes(x = ConesX, y = ConesY), color = "red", size = 0.5) +
  geom_point(data = rod_data, aes(x = RodsX, y = RodsY), color = "blue", size = 0.5) +
  scale_x_continuous(breaks=c(-75,-5,0,5,100))
