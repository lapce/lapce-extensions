-- CreateTable
CREATE TABLE "Plugin" (
    "name" TEXT NOT NULL,
    "description" TEXT NOT NULL,
    "display_name" TEXT NOT NULL,
    "author" TEXT NOT NULL,
    "publisherId" BIGINT NOT NULL,

    CONSTRAINT "Plugin_pkey" PRIMARY KEY ("name")
);

-- CreateTable
CREATE TABLE "Version" (
    "version" TEXT NOT NULL,
    "pluginName" TEXT NOT NULL,
    "yanked" BOOLEAN NOT NULL,
    "digest" TEXT NOT NULL
);

-- CreateIndex
CREATE UNIQUE INDEX "Version_version_pluginName_key" ON "Version"("version", "pluginName");

-- AddForeignKey
ALTER TABLE "Plugin" ADD CONSTRAINT "Plugin_publisherId_fkey" FOREIGN KEY ("publisherId") REFERENCES "User"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "Version" ADD CONSTRAINT "Version_pluginName_fkey" FOREIGN KEY ("pluginName") REFERENCES "Plugin"("name") ON DELETE RESTRICT ON UPDATE CASCADE;
